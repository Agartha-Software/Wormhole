use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::{io, sync::Arc};

use crate::config::local_file::LocalConfigFile;
use crate::config::GlobalConfig;
use crate::error::WhError;
#[cfg(target_os = "linux")]
use crate::fuse::fuse_impl::mount_fuse;
use crate::ipc::{
    self,
    answers::{InspectInfo, PodCreationError},
};
use crate::network;
use crate::network::message::{Request, Response, ToNetworkMessage};
#[cfg(target_os = "linux")]
use crate::pods::disk_managers::unix_disk_manager::UnixDiskManager;
#[cfg(target_os = "windows")]
use crate::pods::disk_managers::windows_disk_manager::WindowsDiskManager;
use crate::pods::itree::creation::{generate_itree, initiate_itree};
use crate::pods::itree::{FsEntry, LOCAL_CONFIG_INO, LOCK_TIMEOUT};
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
use libp2p::{multiaddr, PeerId};
use parking_lot::RwLock;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::pods::{
    filesystem::fs_interface::FsInterface, itree::ITree,
    network::network_interface::NetworkInterface,
};

use super::itree::{Ino, GLOBAL_CONFIG_INO, ITREE_FILE_FNAME, ITREE_FILE_INO};

#[allow(dead_code)]
pub struct Pod {
    pub network_interface: Arc<NetworkInterface>,
    pub fs_interface: Arc<FsInterface>,
    mountpoint: PathBuf,
    #[cfg(target_os = "linux")]
    fuse_handle: fuser::BackgroundSession,
    #[cfg(target_os = "windows")]
    fsp_host: WinfspHost,
    network_airport_handle: JoinHandle<()>,
    redundancy_worker_handle: JoinHandle<()>,
    pub global_config: Arc<RwLock<GlobalConfig>>,
    pub name: String,
    pub nickname: String,
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

impl Pod {
    pub async fn new(
        proto: PodPrototype,
        host_nickname: String,
    ) -> Result<(Self, bool), PodCreationError> {
        let (senders_in, senders_out) = mpsc::unbounded_channel();

        let (redundancy_tx, redundancy_rx) = mpsc::unbounded_channel();

        #[cfg(target_os = "linux")]
        let disk_manager = Box::new(
            UnixDiskManager::new(&proto.mountpoint)
                .map_err(|err| PodCreationError::DiskAccessError(err.into()))?,
        );
        #[cfg(target_os = "windows")]
        let disk_manager = Box::new(
            WindowsDiskManager::new(&proto.mountpoint)
                .map_err(|err| PodCreationError::DiskAccessError(err.into()))?,
        );

        let mut nickname = host_nickname;
        nickname.push(':');
        nickname.push_str(&proto.name);

        let mut swarm = create_swarm(nickname.clone())
            .await
            .map_err(|err| PodCreationError::TransportError(err.to_string()))?;

        for address in proto.listen_addrs {
            swarm
                .listen_on(address)
                .map_err(|err| PodCreationError::TransportError(err.to_string()))?;
        }

        let dialed_success = proto
            .global_config
            .general
            .entrypoints
            .iter()
            .cloned()
            .find_map(|peer| {
                multiaddr::from_url(&format!("ws://{peer}"))
                    .ok()
                    .and_then(|p| swarm.dial(p).ok())
            })
            .is_some();

        let itree = generate_itree(&proto.mountpoint, &swarm.local_peer_id().clone())
            .map_err(|err| PodCreationError::ITreeIndexion(err.into()))?;

        initiate_itree(&itree, &proto.global_config, disk_manager.as_ref())
            .map_err(|err| PodCreationError::ITreeIndexion(err.into()))?;

        let itree = Arc::new(RwLock::new(itree));

        let global = Arc::new(RwLock::new(proto.global_config));

        let network_interface = Arc::new(NetworkInterface::new(
            itree,
            *swarm.local_peer_id(),
            senders_in.clone(),
            redundancy_tx.clone(),
            Arc::new(RwLock::new(swarm.connected_peers().cloned().collect())),
            global.clone(),
        ));

        let fs_interface = Arc::new(FsInterface::new(
            network_interface.clone(),
            disk_manager,
            proto.mountpoint.clone(),
        ));

        let event_loop = EventLoop::new(swarm, fs_interface.clone(), senders_out, dialed_success);

        let network_airport_handle = tokio::spawn(event_loop.run());

        let redundancy_worker_handle = tokio::spawn(redundancy_worker(
            redundancy_rx,
            network_interface.clone(),
            fs_interface.clone(),
        ));

        // FIXME - if mount fuse or fsp errors, drops of disk managers don't seems to be called
        Ok((
            Self {
                network_interface,
                fs_interface: fs_interface.clone(),
                mountpoint: proto.mountpoint.clone(),
                #[cfg(target_os = "linux")]
                fuse_handle: mount_fuse(
                    &proto.mountpoint,
                    proto.allow_other_users,
                    fs_interface.clone(),
                )
                .map_err(|e| {
                    PodCreationError::Mount(
                        io::Error::new(e.kind(), format!("mount_fuse: {e}")).into(),
                    )
                })?,
                #[cfg(target_os = "windows")]
                fsp_host: mount_fsp(fs_interface.clone()).map_err(|e| {
                    PodCreationError::Mount(
                        io::Error::new(e.kind(), format!("mount_fsp: {e}")).into(),
                    )
                })?,
                network_airport_handle,
                global_config: global.clone(),
                redundancy_worker_handle,
                name: proto.name,
                nickname,
                should_restart: proto.should_restart,
                allow_other_users: proto.allow_other_users,
            },
            dialed_success,
        ))
    }

    // SECTION getting info from the pod (for the cli)

    pub fn get_file_hosts(&self, path: &WhPath) -> Result<Vec<PeerId>, PodInfoError> {
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
    // !SECTION

    /// for a given file, will try to send it to one host, trying each until succes
    async fn send_file_to_possible_hosts(
        &self,
        possible_hosts: &[PeerId],
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
            let (status_tx, status_rx) = oneshot::channel();

            self.network_interface
                .to_network_message_tx
                .send(ToNetworkMessage::AnswerMessage(
                    // NOTE - file_content clone is not efficient, but no way to avoid it for now
                    Request::RedundancyFile(ino, file_content.clone()),
                    status_tx,
                    *host,
                ))
                .expect("to_network_message_tx closed.");

            if let Some(Response::Success) = status_rx.await.expect("network died") {
                self.network_interface
                    .to_network_message_tx
                    .send(ToNetworkMessage::BroadcastMessage(Request::RemoveHosts(
                        ino,
                        vec![self.network_interface.id],
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
        peers: Vec<PeerId>,
    ) {
        let ids_to_send = itree
            .files_hosted_only_by(&self.network_interface.id)
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

            let peers: Vec<PeerId> = self.network_interface.peers.read().to_vec();

            let bin = bincode::serialize(&*itree).expect("can't serialize itree to bincode");
            let task = self.send_files_when_stopping(itree, peers);
            (bin, task)
        };
        task.await;

        let Self {
            fs_interface,
            #[cfg(target_os = "linux")]
            fuse_handle,
            #[cfg(target_os = "windows")]
            fsp_host,
            network_airport_handle,
            redundancy_worker_handle,
            ..
        } = self;

        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::CloseNetwork)
            .expect("to_network_message_tx closed.");

        if let Err(err) = network_airport_handle.await {
            log::error!("await error: network_airport_handle: {err}");
        }

        #[cfg(target_os = "linux")]
        drop(fuse_handle);
        #[cfg(target_os = "windows")]
        drop(fsp_host);

        redundancy_worker_handle.abort();
        let _ = redundancy_worker_handle
            .await
            .inspect(|_| log::error!("await error: redundancy_worker_handle"));

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

        let mut fs_interface = Arc::try_unwrap(fs_interface)
            .unwrap_or_else(|_| panic!("fs_interface not released from every thread"));

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
            listen_addrs: self
                .network_interface
                .listen_addrs
                .read()
                .clone()
                .into_iter()
                .collect(),
            mountpoint: self.mountpoint.clone(),
            should_restart: self.should_restart,
            allow_other_users: self.allow_other_users,
        })
    }

    pub fn generate_local_config(&self) -> LocalConfigFile {
        LocalConfigFile {
            name: Some(self.name.clone()),
            restart: Some(self.should_restart),
            listen_addrs: self
                .fs_interface
                .network_interface
                .listen_addrs
                .read()
                .iter()
                .filter_map(|m| network::PeerInfo::display_address(m).ok())
                .collect(),
        }
    }

    pub fn get_inspect_info(&self) -> InspectInfo {
        let listen_addrs = self
            .fs_interface
            .network_interface
            .listen_addrs
            .read()
            .iter()
            .map(|m| network::PeerInfo::display_address(m).unwrap_or_else(|m| m.to_string()))
            .collect();

        let peers_info: Vec<ipc::PeerInfo> = self
            .fs_interface
            .network_interface
            .peers_info
            .read()
            .values()
            .map(Into::into)
            .collect();

        InspectInfo {
            frozen: false,
            listen_addrs,
            name: self.name.clone(),
            connected_peers: peers_info,
            mount: self.mountpoint.clone(),
        }
    }
}
