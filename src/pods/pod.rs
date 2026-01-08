use std::path::{Path, PathBuf};
use std::{io, sync::Arc};

use crate::config::{GlobalConfig, LocalConfig};
use crate::data::tree_hosts::CliHostTree;
use crate::error::{WhError, WhResult};
#[cfg(target_os = "linux")]
use crate::fuse::fuse_impl::mount_fuse;
use crate::ipc::answers::{InspectInfo, PeerInfo};
use crate::network::message::{FromNetworkMessage, MessageContent, ToNetworkMessage};
use crate::network::HandshakeError;
#[cfg(target_os = "linux")]
use crate::pods::disk_managers::unix_disk_manager::UnixDiskManager;
#[cfg(target_os = "windows")]
use crate::pods::disk_managers::windows_disk_manager::WindowsDiskManager;
use crate::pods::disk_managers::DiskManager;
use crate::pods::itree::{
    FsEntry, GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME, LOCAL_CONFIG_INO, LOCK_TIMEOUT, ROOT,
};
use crate::pods::network::redundancy::redundancy_worker;
use crate::pods::whpath::WhPath;
#[cfg(target_os = "windows")]
use crate::winfsp::winfsp_impl::{mount_fsp, WinfspHost};
use custom_error::custom_error;
#[cfg(target_os = "linux")]
use fuser;
use log::info;
use parking_lot::RwLock;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::network::{message::Address, peer_ipc::PeerIPC, server::Server};

use crate::pods::{
    filesystem::fs_interface::FsInterface,
    itree::{generate_itree, ITree},
    network::network_interface::NetworkInterface,
};

use super::itree::{Ino, GLOBAL_CONFIG_INO, ITREE_FILE_FNAME, ITREE_FILE_INO};

#[allow(dead_code)]
#[derive(Debug)]
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
    pub local_config: Arc<RwLock<LocalConfig>>,
}

struct PodPrototype {
    pub itree: ITree,
    pub peers: Vec<PeerIPC>,
    pub global_config: GlobalConfig,
    pub local_config: LocalConfig,
    pub mountpoint: PathBuf,
    pub receiver_out: UnboundedReceiver<FromNetworkMessage>,
    pub receiver_in: UnboundedSender<FromNetworkMessage>,
}

custom_error! {pub PodInfoError
    WhError{source: WhError} = "{source}",
    WrongFileType{detail: String} = "PodInfoError: wrong file type: {detail}",
    FileNotFound = "PodInfoError: file not found",
}

async fn initiate_connection(
    mountpoint: &Path,
    local_config: &LocalConfig,
    global_config: &GlobalConfig,
    receiver_in: &UnboundedSender<FromNetworkMessage>,
    receiver_out: UnboundedReceiver<FromNetworkMessage>,
) -> Result<PodPrototype, UnboundedReceiver<FromNetworkMessage>> {
    if !global_config.general.entrypoints.is_empty() {
        for first_contact in &global_config.general.entrypoints {
            match PeerIPC::connect(first_contact.to_owned(), local_config, receiver_in.clone())
                .await
            {
                Err(HandshakeError::CouldntConnect) => continue,
                Err(e) => {
                    log::error!("{first_contact}: {e}");
                    return Err(receiver_out);
                }
                Ok((ipc, accept)) => {
                    return if let Some(urls) =
                        accept.urls.into_iter().skip(1).fold(Some(vec![]), |a, b| {
                            a.and_then(|mut a| {
                                a.push(b?);
                                Some(a)
                            })
                        }) {
                        let mut local_config = local_config.clone();
                        if let Some(rename) = accept.rename {
                            local_config.general.hostname = rename;
                        }

                        match PeerIPC::peer_startup(
                            urls,
                            local_config.general.hostname.clone(),
                            accept.hostname,
                            receiver_in.clone(),
                        )
                        .await
                        {
                            Ok(mut other_ipc) => {
                                other_ipc.insert(0, ipc);
                                Ok(PodPrototype {
                                    itree: accept.itree,
                                    peers: other_ipc,
                                    global_config: accept.config,
                                    local_config,
                                    mountpoint: mountpoint.into(),
                                    receiver_out,
                                    receiver_in: receiver_in.clone(),
                                })
                            }
                            Err(e) => {
                                log::error!("a peer failed: {e}");
                                Err(receiver_out)
                            }
                        }
                    } else {
                        log::error!("Peers do not all have a url!");
                        Err(receiver_out)
                    };
                }
            }
        }
        info!("None of the known address answered correctly, starting a FS.")
    }
    Err(receiver_out)
}

// fn register_to_others(peers: &Vec<PeerIPC>, self_address: &Address) -> std::io::Result<()> {
//     for peer in peers {
//         peer.sender
//             .send((MessageContent::Register(self_address.clone()), None))
//             .map_err(|err| std::io::Error::new(io::ErrorKind::NotConnected, err))?;
//     }
//     Ok(())
// }

custom_error! {pub PodStopError
    WhError{source: WhError} = "{source}",
    ITreeSavingFailed{source: io::Error} = "Could not write itree to disk: {source}",
    PodNotRunning = "No pod with this name was found running.",
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
    pub async fn new(
        global_config: GlobalConfig,
        local_config: LocalConfig,
        mountpoint: &Path,
        server: Arc<Server>,
    ) -> io::Result<Self> {
        let global_config = global_config;

        log::trace!("mount point {:?}", mountpoint);
        let (receiver_in, receiver_out) = mpsc::unbounded_channel();

        let proto = match initiate_connection(
            mountpoint,
            &local_config,
            &global_config,
            &receiver_in,
            receiver_out,
        )
        .await
        {
            Ok(proto) => proto,
            Err(receiver_out) => {
                if !global_config.general.entrypoints.is_empty() {
                    // NOTE - temporary fix
                    // made to help with tests and debug
                    // choice not to fail should later be supported by the cli
                    log::error!("No peers answered. Stopping.");
                    return Err(io::Error::other("None of the specified peers could answer"));
                }
                let itree = generate_itree(mountpoint, &local_config.general.hostname)
                    .unwrap_or(ITree::new());
                PodPrototype {
                    itree,
                    peers: vec![],
                    global_config,
                    local_config,
                    mountpoint: mountpoint.into(),
                    receiver_out,
                    receiver_in,
                }
            }
        };

        Self::realize(proto, server).await
    }

    async fn realize(proto: PodPrototype, server: Arc<Server>) -> io::Result<Self> {
        let (senders_in, senders_out) = mpsc::unbounded_channel();

        let (redundancy_tx, redundancy_rx) = mpsc::unbounded_channel();

        #[cfg(target_os = "linux")]
        let disk_manager = Box::new(UnixDiskManager::new(&proto.mountpoint)?);
        #[cfg(target_os = "windows")]
        let disk_manager = Box::new(WindowsDiskManager::new(&proto.mountpoint)?);

        create_all_shared(&proto.itree, ROOT, disk_manager.as_ref())
            .inspect_err(|e| log::error!("unable to create_all_shared: {e}"))
            .map_err(|e| std::io::Error::new(e.kind(), format!("create_all_shared: {e}")))?;

        if let Ok(perms) = proto
            .itree
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

        if let Ok(perms) = proto
            .itree
            .get_inode(LOCAL_CONFIG_INO)
            .map(|inode| inode.meta.perm)
        {
            let _ = disk_manager.new_file(&WhPath::try_from(LOCAL_CONFIG_FNAME).unwrap(), perms);
            disk_manager
                .write_file(
                    &WhPath::try_from(LOCAL_CONFIG_FNAME).unwrap(),
                    toml::to_string(&proto.local_config)
                        .expect("infallible")
                        .as_bytes(),
                    0,
                )
                .map_err(|e| {
                    std::io::Error::new(e.kind(), format!("write_file(local_config): {e}"))
                })?;
        }

        let url = proto.local_config.general.url.clone();

        let itree: Arc<RwLock<ITree>> = Arc::new(RwLock::new(proto.itree));
        let local = Arc::new(RwLock::new(proto.local_config));
        let global = Arc::new(RwLock::new(proto.global_config));

        let network_interface = Arc::new(NetworkInterface::new(
            itree.clone(),
            url,
            senders_in.clone(),
            redundancy_tx.clone(),
            Arc::new(RwLock::new(proto.peers)),
            local.clone(),
            global.clone(),
        ));

        let fs_interface = Arc::new(FsInterface::new(
            network_interface.clone(),
            disk_manager,
            itree.clone(),
            proto.mountpoint.clone(),
        ));

        // Start ability to recieve messages
        let network_airport_handle = tokio::spawn(NetworkInterface::network_airport(
            proto.receiver_out,
            fs_interface.clone(),
        ));

        // Start ability to send messages
        let peer_broadcast_handle = tokio::spawn(NetworkInterface::contact_peers(
            network_interface.peers.clone(),
            senders_out,
        ));

        let new_peer_handle = tokio::spawn(NetworkInterface::incoming_connections_watchdog(
            server,
            proto.receiver_in.clone(),
            network_interface.clone(),
        ));

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
            fuse_handle: mount_fuse(&proto.mountpoint, fs_interface.clone())
                .map_err(|e| std::io::Error::new(e.kind(), format!("mount_fuse: {e}")))?,
            #[cfg(target_os = "windows")]
            fsp_host: mount_fsp(&proto.mountpoint, fs_interface.clone())
                .map_err(|e| std::io::Error::new(e.kind(), format!("mount_fsp: {e}")))?,
            network_airport_handle,
            peer_broadcast_handle,
            new_peer_handle,
            local_config: local.clone(),
            global_config: global.clone(),
            redundancy_worker_handle,
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
                        MessageContent::RedundancyFile(ino, file_content.clone()),
                        Some(status_tx.clone()),
                    ),
                    vec![host.clone()],
                ))
                .expect("to_network_message_tx closed.");

            if let Some(Ok(())) = status_rx.recv().await {
                self.network_interface
                    .to_network_message_tx
                    .send(ToNetworkMessage::BroadcastMessage(
                        MessageContent::EditHosts(ino, vec![host.clone()]),
                    ))
                    .expect("to_network_message_tx closed.");
                return Ok(());
            }
        }
        Err(PodStopError::FileNotSent { file: ino })
    }

    /// Gets every file hosted by this pod only and sends them to other pods
    async fn send_files_when_stopping(&self, itree: &ITree, peers: Vec<Address>) {
        futures_util::future::join_all(
            itree
                .files_hosted_only_by(
                    &self
                        .network_interface
                        .local_config
                        .read()
                        .general
                        .hostname
                        .clone(),
                )
                .filter_map(|inode| {
                    if inode.id == GLOBAL_CONFIG_INO
                        || inode.id == LOCAL_CONFIG_INO
                        || inode.id == ITREE_FILE_INO
                    {
                        None
                    } else {
                        Some(inode.id)
                    }
                })
                .map(|id| self.send_file_to_possible_hosts(&peers, id)),
        )
        .await
        .iter()
        .for_each(|e| {
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

        let itree = ITree::read_lock(&self.network_interface.itree, "Pod::Pod::stop(1)")?;

        let peers: Vec<Address> = self
            .peers
            .read()
            .iter()
            .map(|peer| peer.hostname.clone())
            .collect();

        self.send_files_when_stopping(&itree, peers).await;
        let itree_bin = bincode::serialize(&*itree).expect("can't serialize itree to bincode");
        drop(itree);

        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::Disconnect,
            ))
            .expect("to_network_message_tx closed.");

        let Self {
            network_interface: _,
            fs_interface,
            mountpoint: _,
            peers,
            #[cfg(target_os = "linux")]
            fuse_handle,
            #[cfg(target_os = "windows")]
            fsp_host,
            network_airport_handle,
            peer_broadcast_handle,
            new_peer_handle,
            redundancy_worker_handle,
            global_config: _,
            local_config: _,
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

    pub fn contains(&self, path: &PathBuf) -> bool {
        path.starts_with(&self.mountpoint)
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
            url: self.network_interface.url.clone(),
            hostname: self
                .network_interface
                .hostname()
                .expect("Can't lock network"),
            name: "".to_string(), //TODO to delete
            connected_peers: peers_data,
            mount: self.mountpoint.clone(),
        }
    }
}
