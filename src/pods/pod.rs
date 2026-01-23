use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::{io, sync::Arc};

use crate::config::local_file::LocalConfigFile;
use crate::config::GlobalConfig;
use crate::data::tree_hosts::CliHostTree;
use crate::error::{WhError, WhResult};
#[cfg(target_os = "linux")]
use crate::fuse::fuse_impl::mount_fuse;
use crate::ipc::answers::{InspectInfo, PeerInfo};
use crate::network::message::{FromNetworkMessage, Request, ToNetworkMessage};
#[cfg(target_os = "linux")]
use crate::pods::disk_managers::unix_disk_manager::UnixDiskManager;
#[cfg(target_os = "windows")]
use crate::pods::disk_managers::windows_disk_manager::WindowsDiskManager;
use crate::pods::disk_managers::DiskManager;
use crate::pods::itree::{FsEntry, GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_INO, LOCK_TIMEOUT, ROOT};
use crate::pods::network::callbacks::Callbacks;
use crate::pods::network::event_loop::EventLoop;
use crate::pods::network::redundancy::redundancy_worker;
use crate::pods::network::swarm::create_swarm;
use crate::pods::prototype::PodPrototype;
use crate::pods::whpath::WhPath;
#[cfg(target_os = "windows")]
use crate::winfsp::winfsp_impl::{mount_fsp, WinfspHost};
use custom_error::custom_error;
#[cfg(target_os = "linux")]
use fuser;
use libp2p::swarm;
use parking_lot::RwLock;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::network::{message::Address, peer_ipc::PeerIPC, server::Server};

use crate::pods::{
    filesystem::fs_interface::FsInterface, itree::ITree,
    network::network_interface::NetworkInterface,
};

use super::itree::{Ino, GLOBAL_CONFIG_INO, ITREE_FILE_FNAME, ITREE_FILE_INO};

#[allow(dead_code)]
pub struct Pod {
    network_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
    mountpoint: PathBuf,
    pub peers: Arc<RwLock<Vec<PeerIPC>>>,
    #[cfg(target_os = "linux")]
    fuse_handle: fuser::BackgroundSession,
    #[cfg(target_os = "windows")]
    fsp_host: WinfspHost,
    network_airport_handle: JoinHandle<()>,
    peer_broadcast_handle: JoinHandle<()>,
    new_peer_handle: JoinHandle<()>,
    redundancy_worker_handle: JoinHandle<()>,
    pub global_config: Arc<RwLock<GlobalConfig>>,
    name: String,
    pub should_restart: bool,
    allow_other_users: bool,
}

custom_error! {pub PodInfoError
    WhError{source: WhError} = "{source}",
    WrongFileType{detail: String} = "PodInfoError: wrong file type: {detail}",
    FileNotFound = "PodInfoError: file not found",
}

custom_error! {pub PodStopError
    WhError{source: WhError} = "{source}",
    ITreeSavingFailed{source: io::Error} = "Could not write itree to disk: {source}",
    FileNotReadable{file: Ino, reason: String} = "Could not read file from disk: ({file}) {reason}",
    FileNotSent{file: Ino} = "No pod was able to receive this file before stopping: ({file})",
    #[cfg(target_os = "linux")]
    DiskManagerStopFailed{e: io::Error} = "Unable to stop the disk manager properly. Should not be an error on your platform {e}",
    #[cfg(target_os = "windows")]
    DiskManagerStopFailed{e: io::Error} = "Unable to stop the disk manager properly. Your files are still on the system folder: ('.'mount_path). {e}",
}

/// Create all directories and symlinks present in ITree. (not the files)
///
/// Required at setup to resolve issue #179
/// (files pulling need the parent folder to be already present)
fn create_all_shared(itree: &ITree, from: Ino, disk: &dyn DiskManager) -> io::Result<()> {
    let from = itree.get_inode(from).map_err(|e| e.into_io())?;

    match &from.entry {
        FsEntry::File(_) => Ok(()),
        FsEntry::Symlink(symlink) => {
            let current_path = itree
                .get_path_from_inode_id(from.id)
                .map_err(|e| e.into_io())?;
            disk.new_symlink(&current_path, from.meta.perm, symlink)
                .or_else(|e| {
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        Ok(())
                    } else {
                        Err(e)
                    }
                })
        }
        FsEntry::Directory(children) => {
            let current_path = itree
                .get_path_from_inode_id(from.id)
                .map_err(|e| e.into_io())?;

            // skipping root folder
            if current_path != WhPath::root() {
                disk.new_dir(&current_path, from.meta.perm).or_else(|e| {
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        Ok(())
                    } else {
                        Err(e)
                    }
                })?;
            }

            for child in children {
                create_all_shared(itree, *child, disk)?
            }
            Ok(())
        }
    }
}

impl Pod {
    pub async fn new(mut proto: PodPrototype, server: Arc<Server>) -> io::Result<Self> {
        let (senders_in, senders_out) = mpsc::unbounded_channel();

        let (redundancy_tx, redundancy_rx) = mpsc::unbounded_channel();

        #[cfg(target_os = "linux")]
        let disk_manager = Box::new(UnixDiskManager::new(&proto.mountpoint)?);
        #[cfg(target_os = "windows")]
        let disk_manager = Box::new(WindowsDiskManager::new(&proto.mountpoint)?);

        create_all_shared(&itree, ROOT, disk_manager.as_ref())
            .inspect_err(|e| log::error!("unable to create_all_shared: {e}"))
            .map_err(|e| std::io::Error::new(e.kind(), format!("create_all_shared: {e}")))?;

        if let Ok(perms) = itree
            .get_inode(GLOBAL_CONFIG_INO)
            .map(|inode| inode.meta.perm)
        {
            let _ = disk_manager.new_file(&WhPath::try_from(GLOBAL_CONFIG_FNAME).unwrap(), perms);
            disk_manager
                .write_file(
                    &WhPath::try_from(GLOBAL_CONFIG_FNAME).unwrap(),
                    toml::to_string(&proto.global_config)
                        .expect("infallible")
                        .as_bytes(),
                    0,
                )
                .map_err(|e| {
                    std::io::Error::new(e.kind(), format!("write_file(global_config): {e}"))
                })?;
        }

        let itree: Arc<RwLock<ITree>> = Arc::new(RwLock::new(itree));
        let global = Arc::new(RwLock::new(proto.global_config));

        let network_interface = Arc::new(NetworkInterface::new(
            itree.clone(),
            proto.public_url,
            proto.bound_socket,
            proto.hostname,
            senders_in.clone(),
            redundancy_tx.clone(),
            Arc::new(RwLock::new(peers)),
            global.clone(),
        ));

        let fs_interface = Arc::new(FsInterface::new(
            network_interface.clone(),
            disk_manager,
            itree.clone(),
            proto.mountpoint.clone(),
        ));

        let swarm = create_swarm(proto.name).await.unwrap();
        swarm.listen_on(addr);
        swarm....

        let event_loop = EventLoop::new(fs_interface, senders_out).await.unwrap();

        let network_airport_handle = tokio::spawn(event_loop.run());

        let peers = network_interface.peers.clone();

        let redundancy_worker_handle = tokio::spawn(redundancy_worker(
            redundancy_rx,
            network_interface.clone(),
            fs_interface.clone(),
        ));

        // FIXME - if mount fuse or fsp errors, drops of disk managers don't seems to be called
        Ok(Self {
            network_interface,
            fs_interface: fs_interface.clone(),
            mountpoint: proto.mountpoint.clone(),
            peers,
            #[cfg(target_os = "linux")]
            fuse_handle: mount_fuse(
                &proto.mountpoint,
                proto.allow_other_users,
                fs_interface.clone(),
            )
            .map_err(|e| std::io::Error::new(e.kind(), format!("mount_fuse: {e}")))?,
            #[cfg(target_os = "windows")]
            fsp_host: mount_fsp(fs_interface.clone())
                .map_err(|e| std::io::Error::new(e.kind(), format!("mount_fsp: {e}")))?,
            network_airport_handle,
            peer_broadcast_handle,
            new_peer_handle,
            global_config: global.clone(),
            redundancy_worker_handle,
            name: proto.name,
            should_restart: proto.should_restart,
            allow_other_users: proto.allow_other_users,
        })
    }

    // SECTION getting info from the pod (for the cli)

    pub fn get_file_hosts(&self, path: &WhPath) -> Result<Vec<Address>, PodInfoError> {
        let binding = ITree::read_lock(&self.network_interface.itree, "Pod::get_info")?;
        let entry = &binding
            .get_inode_from_path(path)
            .map_err(|_| PodInfoError::FileNotFound)?
            .entry;

        match entry {
            FsEntry::File(hosts) => Ok(hosts.clone()),
            _ => Err(PodInfoError::WrongFileType {
                detail: "Requested path not a file (only files have hosts)".to_owned(),
            }),
        }
    }

    pub fn get_file_tree_and_hosts(
        &self,
        path: Option<&WhPath>,
    ) -> Result<CliHostTree, PodInfoError> {
        let itree = ITree::read_lock(&self.network_interface.itree, "Pod::get_info")?;

        Ok(CliHostTree {
            lines: itree.get_file_tree_and_hosts(path)?,
        })
    }

    // !SECTION

    /// for a given file, will try to send it to one host, trying each until succes
    async fn send_file_to_possible_hosts(
        &self,
        possible_hosts: &Vec<Address>,
        ino: Ino,
    ) -> Result<(), PodStopError> {
        let file_content =
            self.fs_interface
                .read_local_file(ino)
                .map_err(|e| PodStopError::FileNotReadable {
                    file: ino,
                    reason: e.to_string(),
                })?;
        let file_content = Arc::new(file_content);

        for host in possible_hosts {
            let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel::<WhResult<()>>();

            self.network_interface
                .to_network_message_tx
                .send(ToNetworkMessage::SpecificMessage(
                    (
                        // NOTE - file_content clone is not efficient, but no way to avoid it for now
                        Request::RedundancyFile(ino, file_content.clone()),
                        Some(status_tx.clone()),
                    ),
                    vec![host.clone()],
                ))
                .expect("to_network_message_tx closed.");

            if let Some(Ok(())) = status_rx.recv().await {
                self.network_interface
                    .to_network_message_tx
                    .send(ToNetworkMessage::BroadcastMessage(Request::EditHosts(
                        ino,
                        vec![host.clone()],
                    )))
                    .expect("to_network_message_tx closed.");
                return Ok(());
            }
        }
        Err(PodStopError::FileNotSent { file: ino })
    }

    /// Gets every file hosted by this pod only and sends them to other pods
    async fn send_files_when_stopping<T: Deref<Target = ITree>>(
        &self,
        itree: T,
        peers: Vec<Address>,
    ) {
        let ids_to_send = itree
            .files_hosted_only_by(&self.network_interface.hostname)
            .filter_map(|inode| {
                if inode.id == GLOBAL_CONFIG_INO
                    || inode.id == LOCAL_CONFIG_INO
                    || inode.id == ITREE_FILE_INO
                {
                    None
                } else {
                    Some(inode.id)
                }
            });
        let tasks = futures_util::future::join_all(
            ids_to_send.map(|id| self.send_file_to_possible_hosts(&peers, id)),
        );
        drop(itree);
        tasks.await.iter().for_each(|e| {
            if let Err(e) = e {
                log::warn!("{e:?}")
            }
        });
    }

    pub async fn stop(self) -> Result<(), PodStopError> {
        // TODO
        // in actual state, all operations (request from network other than just pulling the asked files)
        // made after calling this function but before dropping the pod are undefined behavior.

        // drop(self.fuse_handle); // FIXME - do something like block the filesystem

        // moving the await task outside the scope needed to workaround https://github.com/rust-lang/rust-clippy/issues/6446
        let (itree_bin, task) = {
            let itree = ITree::read_lock(&self.network_interface.itree, "Pod::Pod::stop(1)")?;

            let peers: Vec<Address> = self
                .peers
                .read()
                .iter()
                .map(|peer| peer.hostname.clone())
                .collect();

            let bin = bincode::serialize(&*itree).expect("can't serialize itree to bincode");
            let task = self.send_files_when_stopping(itree, peers);
            (bin, task)
        };
        task.await;

        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(Request::Disconnect))
            .expect("to_network_message_tx closed.");

        let Self {
            fs_interface,
            peers,
            #[cfg(target_os = "linux")]
            fuse_handle,
            #[cfg(target_os = "windows")]
            fsp_host,
            network_airport_handle,
            peer_broadcast_handle,
            new_peer_handle,
            redundancy_worker_handle,
            ..
        } = self;

        #[cfg(target_os = "linux")]
        drop(fuse_handle);
        #[cfg(target_os = "windows")]
        drop(fsp_host);

        redundancy_worker_handle.abort();
        let _ = redundancy_worker_handle
            .await
            .inspect(|_| log::error!("await error: redundancy_worker_handle"));
        network_airport_handle.abort();
        let _ = network_airport_handle
            .await
            .inspect(|_| log::error!("await error: network_airport_handle"));
        new_peer_handle.abort();
        let _ = new_peer_handle
            .await
            .inspect(|_| log::error!("await error: new_peer_handle"));
        peer_broadcast_handle.abort();
        let _ = peer_broadcast_handle
            .await
            .inspect(|_| log::error!("await error: peer_broadcast_handle"));
        *peers.write() = Vec::new(); // dropping PeerIPCs

        let itree_path = WhPath::try_from(ITREE_FILE_FNAME).unwrap();

        if !fs_interface.disk.file_exists(&itree_path) {
            fs_interface
                .disk
                .new_file(&itree_path, 0o600) // REVIEW - permissions value ?
                .map_err(|io| PodStopError::ITreeSavingFailed { source: io })?;
        }

        fs_interface
            .disk
            .write_file(&WhPath::try_from(ITREE_FILE_FNAME).unwrap(), &itree_bin, 0)
            .map_err(|io| PodStopError::ITreeSavingFailed { source: io })?;

        let mut fs_interface =
            Arc::try_unwrap(fs_interface).expect("fs_interface not released from every thread");

        fs_interface
            .disk
            .stop()
            .map_err(|e| PodStopError::DiskManagerStopFailed { e })?;

        Ok(())
    }

    pub fn get_mountpoint(&self) -> &PathBuf {
        &self.mountpoint
    }

    pub fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.mountpoint)
    }

    pub fn try_generate_prototype(&self) -> Option<PodPrototype> {
        let global_config = self.global_config.try_read_for(LOCK_TIMEOUT)?.clone();

        Some(PodPrototype {
            global_config,
            name: self.name.clone(),
            hostname: self.network_interface.hostname.clone(),
            public_url: self.network_interface.public_url.clone(),
            bound_socket: self.network_interface.bound_socket,
            mountpoint: self.mountpoint.clone(),
            should_restart: self.should_restart,
            allow_other_users: self.allow_other_users,
        })
    }

    pub fn generate_local_config(&self) -> LocalConfigFile {
        LocalConfigFile {
            name: Some(self.name.clone()),
            hostname: Some(self.network_interface.hostname.clone()),
            public_url: self.network_interface.public_url.clone(),
            restart: Some(self.should_restart),
        }
    }

    pub fn get_inspect_info(&self) -> InspectInfo {
        let peers_data: Vec<PeerInfo> = self
            .peers
            .try_read_for(LOCK_TIMEOUT)
            .expect("Can't lock peers")
            .iter()
            .map(|peer| PeerInfo {
                hostname: peer.hostname.clone(),
                url: peer.url.clone(),
            })
            .collect();

        InspectInfo {
            frozen: false,
            public_url: self.network_interface.public_url.clone(),
            bound_socket: self.network_interface.bound_socket,
            hostname: self.network_interface.hostname.clone(),
            name: self.name.clone(),
            connected_peers: peers_data,
            mount: self.mountpoint.clone(),
        }
    }
}
