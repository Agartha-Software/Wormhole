use std::time::Duration;
use std::{io, sync::Arc};

use crate::config::{GlobalConfig, LocalConfig};
use crate::data::tree_hosts::{CliHostTree, TreeLine};
use crate::error::{WhError, WhResult};
#[cfg(target_os = "linux")]
use crate::fuse::fuse_impl::mount_fuse;
use crate::network::message::{
    FileSystemSerialized, FromNetworkMessage, MessageContent, ToNetworkMessage,
};
use crate::network::HandshakeError;
use crate::pods::arbo::{FsEntry, GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME, LOCAL_CONFIG_INO, ROOT};
#[cfg(target_os = "windows")]
use crate::pods::disk_managers::dummy_disk_manager::DummyDiskManager;
#[cfg(target_os = "linux")]
use crate::pods::disk_managers::unix_disk_manager::UnixDiskManager;
use crate::pods::disk_managers::DiskManager;
use crate::pods::network::redundancy::redundancy_worker;
#[cfg(target_os = "windows")]
use crate::winfsp::winfsp_impl::{mount_fsp, WinfspHost};
use custom_error::custom_error;
#[cfg(target_os = "linux")]
use fuser;
use futures::future::Either;
use log::info;
use parking_lot::RwLock;
use serde::Serialize;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::network::{message::Address, peer_ipc::PeerIPC, server::Server};

use crate::pods::{
    arbo::{generate_arbo, Arbo},
    filesystem::fs_interface::FsInterface,
    network::network_interface::NetworkInterface,
    whpath::WhPath,
};

use super::arbo::{InodeId, ARBO_FILE_FNAME, ARBO_FILE_INO, GLOBAL_CONFIG_INO};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Pod {
    network_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
    mountpoint: WhPath,
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
    pub arbo: Arbo,
    pub peers: Vec<PeerIPC>,
    pub global_config: GlobalConfig,
    pub local_config: LocalConfig,
    pub mountpoint: WhPath,
    pub receiver_out: UnboundedReceiver<FromNetworkMessage>,
    pub receiver_in: UnboundedSender<FromNetworkMessage>,
}

custom_error! {pub PodInfoError
    WhError{source: WhError} = "{source}",
    WrongFileType{detail: String} = "PodInfoError: wrong file type: {detail}",
    FileNotFound = "PodInfoError: file not found",
}

async fn initiate_connection(
    mountpoint: &WhPath,
    local_config: &LocalConfig,
    global_config: &GlobalConfig,
    receiver_in: &UnboundedSender<FromNetworkMessage>,
    receiver_out: UnboundedReceiver<FromNetworkMessage>,
) -> Result<PodPrototype, UnboundedReceiver<FromNetworkMessage>> {
    if global_config.general.entrypoints.len() >= 1 {
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
                                    arbo: accept.arbo,
                                    peers: other_ipc,
                                    global_config: accept.config,
                                    local_config,
                                    mountpoint: mountpoint.clone(),
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
    ArboSavingFailed{source: io::Error} = "PodStopError: could not write arbo to disk: {source}",
    PodNotRunning = "No pod with this name was found running.",
    FileNotReadable{file: InodeId, reason: String} = "PodStopError: could not read file from disk: ({file}) {reason}",
    FileNotSent{file: InodeId} = "PodStopError: no pod was able to receive this file before stopping: ({file})"
}

/// Create all the directories present in Arbo. (not the files)
///
/// Required at setup to resolve issue #179
/// (files pulling need the parent folder to be already present)
fn create_all_dirs(arbo: &Arbo, from: InodeId, disk: &dyn DiskManager) -> io::Result<()> {
    let from = arbo.n_get_inode(from).map_err(|e| e.into_io())?;

    return match &from.entry {
        FsEntry::File(_) => Ok(()),
        FsEntry::Directory(children) => {
            let current_path = arbo
                .n_get_path_from_inode_id(from.id)
                .map_err(|e| e.into_io())?;
            disk.new_dir(&current_path, from.meta.perm).or_else(|e| {
                if e.kind() == io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(e)
                }
            })?;

            for child in children {
                create_all_dirs(arbo, *child, disk)?
            }
            Ok(())
        }
    };
}

impl Pod {
    pub async fn new(
        global_config: GlobalConfig,
        local_config: LocalConfig,
        mountpoint: WhPath,
        server: Arc<Server>,
    ) -> io::Result<Self> {
        let global_config = global_config;

        log::trace!("mount point {}", mountpoint);
        let (receiver_in, receiver_out) = mpsc::unbounded_channel();

        let proto = match initiate_connection(
            &mountpoint,
            &local_config,
            &global_config,
            &receiver_in,
            receiver_out,
        )
        .await
        {
            Ok(proto) => proto,
            Err(receiver_out) => {
                if global_config.general.entrypoints.len() > 0 {
                    // NOTE - temporary fix
                    // made to help with tests and debug
                    // choice not to fail should later be supported by the cli
                    log::error!("No peers answered. Stopping.");
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "None of the specified peers could answer",
                    ));
                }
                let arbo = generate_arbo(&mountpoint, &local_config.general.hostname)
                    .unwrap_or(Arbo::new());
                PodPrototype {
                    arbo,
                    peers: vec![],
                    global_config,
                    local_config,
                    mountpoint,
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
        let disk_manager = Box::new(DummyDiskManager::new(&proto.mountpoint)?);

        create_all_dirs(&proto.arbo, ROOT, disk_manager.as_ref())
            .inspect_err(|e| log::error!("unable to create_all_dirs: {e}"))?;

        if let Ok(perms) = proto
            .arbo
            .get_inode(GLOBAL_CONFIG_INO)
            .map(|inode| inode.meta.perm)
        {
            let _ = disk_manager.new_file(&GLOBAL_CONFIG_FNAME.into(), perms);
            disk_manager.write_file(
                &GLOBAL_CONFIG_FNAME.into(),
                toml::to_string(&proto.global_config)
                    .expect("infallible")
                    .as_bytes(),
                0,
            )?;
        }

        if let Ok(perms) = proto
            .arbo
            .get_inode(LOCAL_CONFIG_INO)
            .map(|inode| inode.meta.perm)
        {
            let _ = disk_manager.new_file(&LOCAL_CONFIG_FNAME.into(), perms);
            disk_manager.write_file(
                &LOCAL_CONFIG_FNAME.into(),
                toml::to_string(&proto.local_config)
                    .expect("infallible")
                    .as_bytes(),
                0,
            )?;
        }

        let url = proto.local_config.general.url.clone();

        let arbo: Arc<RwLock<Arbo>> = Arc::new(RwLock::new(proto.arbo));
        let local = Arc::new(RwLock::new(proto.local_config));
        let global = Arc::new(RwLock::new(proto.global_config));

        let network_interface = Arc::new(NetworkInterface::new(
            arbo.clone(),
            proto.mountpoint.clone(),
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
            arbo.clone(),
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

        Ok(Self {
            network_interface,
            fs_interface: fs_interface.clone(),
            mountpoint: proto.mountpoint.clone(),
            peers,
            #[cfg(target_os = "linux")]
            fuse_handle: mount_fuse(&proto.mountpoint, fs_interface.clone())?,
            #[cfg(target_os = "windows")]
            fsp_host: mount_fsp(&mountpoint, fs_interface.clone())?,
            network_airport_handle,
            peer_broadcast_handle,
            new_peer_handle,
            local_config: local.clone(),
            global_config: global.clone(),
            redundancy_worker_handle,
        })
    }

    // SECTION getting info from the pod (for the cli)

    pub fn get_file_hosts(&self, path: WhPath) -> Result<Vec<Address>, PodInfoError> {
        let entry = Arbo::n_read_lock(&self.network_interface.arbo, "Pod::get_info")?
            .get_inode_from_path(&path)
            .map_err(|_| PodInfoError::FileNotFound)?
            .entry
            .clone();

        match entry {
            FsEntry::File(hosts) => Ok(hosts),
            FsEntry::Directory(_) => Err(PodInfoError::WrongFileType {
                detail: "Asked path is a directory (directories have no hosts)".to_owned(),
            }),
        }
    }

    pub fn get_file_tree_and_hosts(
        &self,
        path: Option<WhPath>,
    ) -> Result<CliHostTree, PodInfoError> {
        let arbo = Arbo::n_read_lock(&self.network_interface.arbo, "Pod::get_info")?;
        let ino = if let Some(path) = path {
            arbo.get_inode_from_path(&path)
                .map_err(|_| PodInfoError::FileNotFound)?
                .id
        } else {
            ROOT
        };

        Ok(CliHostTree {
            lines: Self::recurse_tree(&*arbo, ino, 0),
        })
    }

    /// given ino is not checked -> must exist in arbo
    fn recurse_tree(arbo: &Arbo, ino: InodeId, indentation: u8) -> Vec<TreeLine> {
        let entry = &arbo
            .n_get_inode(ino)
            .expect("recurse_tree: ino not found")
            .entry;
        let path = arbo
            .n_get_path_from_inode_id(ino)
            .expect("recurse_tree: unable to get path");
        match entry {
            FsEntry::File(hosts) => vec![(indentation, ino, path, hosts.clone())],
            FsEntry::Directory(children) => children
                .iter()
                .map(|c| Pod::recurse_tree(arbo, *c, indentation + 1))
                .flatten()
                .collect::<Vec<TreeLine>>(),
        }
    }

    // !SECTION

    /// for a given file, will try to send it to one host, trying each until succes
    async fn send_file_to_possible_hosts(
        &self,
        possible_hosts: &Vec<Address>,
        ino: InodeId,
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
    async fn send_files_when_stopping(&self, arbo: &Arbo, peers: Vec<Address>) {
        futures_util::future::join_all(
            arbo.files_hosted_only_by(
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
                    || inode.id == ARBO_FILE_INO
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

        let arbo = Arbo::n_read_lock(&self.network_interface.arbo, "Pod::Pod::stop(1)")?;

        let peers: Vec<Address> = self
            .peers
            .read()
            .iter()
            .map(|peer| peer.hostname.clone())
            .collect();

        self.send_files_when_stopping(&arbo, peers).await;
        let arbo_bin = bincode::serialize(&*arbo).expect("can't serialize arbo to bincode");
        drop(arbo);

        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::Disconnect(
                    self.network_interface
                        .local_config
                        .read()
                        .general
                        .hostname
                        .clone(),
                ),
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
            redundancy_worker_handle: _,
            global_config: _,
            local_config: _,
        } = self;

        #[cfg(target_os = "linux")]
        drop(fuse_handle);
        #[cfg(target_os = "windows")]
        drop(fsp_host);

        fs_interface
            .disk
            .write_file(&ARBO_FILE_FNAME.into(), &arbo_bin, 0)
            .map_err(|io| PodStopError::ArboSavingFailed { source: io })?;

        *peers.write() = Vec::new(); // dropping PeerIPCs
        network_airport_handle.abort();
        new_peer_handle.abort();
        peer_broadcast_handle.abort();
        Ok(())
    }

    pub fn get_mountpoint(&self) -> &WhPath {
        return &self.mountpoint;
    }
}
