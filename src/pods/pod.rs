use std::fs;
use std::{io, sync::Arc};

use crate::config::types::Config;
use crate::config::{GlobalConfig, LocalConfig};
use crate::data::tree_hosts::{CliHostTree, TreeLine};
use crate::error::{WhError, WhResult};
#[cfg(target_os = "linux")]
use crate::fuse::fuse_impl::mount_fuse;
use crate::network::message::{
    FileSystemSerialized, FromNetworkMessage, MessageContent, ToNetworkMessage,
};
use crate::pods::arbo::{FsEntry, LOCAL_CONFIG_FNAME, LOCAL_CONFIG_INO, ROOT};
#[cfg(target_os = "windows")]
use crate::pods::disk_managers::dummy_disk_manager::DummyDiskManager;
#[cfg(target_os = "linux")]
use crate::pods::disk_managers::unix_disk_manager::UnixDiskManager;
use crate::pods::network::redundancy::redundancy_worker;
#[cfg(target_os = "windows")]
use crate::winfsp::winfsp_impl::mount_fsp;
use custom_error::custom_error;
#[cfg(target_os = "linux")]
use fuser;
use log::info;
use parking_lot::RwLock;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
#[cfg(target_os = "windows")]
use winfsp::host::FileSystemHost;

#[cfg(target_os = "linux")]
use crate::pods::arbo::GLOBAL_CONFIG_FNAME;

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
    name: String,
    network_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
    mount_point: WhPath,
    peers: Arc<RwLock<Vec<PeerIPC>>>,
    #[cfg(target_os = "linux")]
    fuse_handle: fuser::BackgroundSession,
    #[cfg(target_os = "windows")]
    fsp_host: FileSystemHost<'static>,
    network_airport_handle: JoinHandle<()>,
    peer_broadcast_handle: JoinHandle<()>,
    new_peer_handle: JoinHandle<()>,
    redundancy_worker_handle: JoinHandle<()>,
    pub global_config: Arc<RwLock<GlobalConfig>>,
    pub local_config: Arc<RwLock<LocalConfig>>,
}

custom_error! {pub PodInfoError
    WhError{source: WhError} = "{source}",
    WrongFileType{detail: String} = "PodInfoError: wrong file type: {detail}",
    FileNotFound = "PodInfoError: file not found",
}

pub async fn initiate_connection(
    peers_addrs: Vec<Address>,
    server_address: Address,
    tx: &UnboundedSender<FromNetworkMessage>,
    rx: &mut UnboundedReceiver<FromNetworkMessage>,
) -> Option<(FileSystemSerialized, Vec<Address>, PeerIPC, Vec<u8>)> {
    if peers_addrs.len() >= 1 {
        for first_contact in peers_addrs {
            let first_ipc = PeerIPC::connect(first_contact.to_owned(), tx.clone()).await;

            if let Some(ipc) = first_ipc {
                if let Err(err) = ipc.sender.send((MessageContent::RequestFs, None)) {
                    info!(
                        "Connection with {first_contact} failed: {err}.\n
                        Trying with next know address"
                    );
                    continue;
                }

                loop {
                    match rx.recv().await {
                        Some(FromNetworkMessage {
                            origin: _,
                            content: MessageContent::FsAnswer(fs, mut peers_address, global_config),
                        }) => {
                            // remove itself from peers and first_connect because the connection is already existing
                            peers_address.retain(|address| {
                                *address != server_address && *address != first_contact
                            });
                            return Some((fs, peers_address, ipc, global_config));
                        }
                        Some(_) => {
                            info!(
                                "First message with {first_contact} failed: His answer is not the FileSystem, corrupted client.\n
                                Trying with next know address"
                            );
                            break;
                        }
                        None => continue,
                    };
                }
            }
        }
        info!("None of the known address answered correctly, starting a FS.")
    }
    None
}

fn register_to_others(peers: &Vec<PeerIPC>, self_address: &Address) -> std::io::Result<()> {
    for peer in peers {
        peer.sender
            .send((MessageContent::Register(self_address.clone()), None))
            .map_err(|err| std::io::Error::new(io::ErrorKind::NotConnected, err))?;
    }
    Ok(())
}

custom_error! {pub PodStopError
    WhError{source: WhError} = "{source}",
    ArboSavingFailed{error_source: String} = "PodStopError: could not write arbo to disk: {error_source}",
    PodNotRunning = "No pod with this name was found running.",
    FileNotReadable{file: InodeId, reason: String} = "PodStopError: could not read file from disk: ({file}) {reason}",
    FileNotSent{file: InodeId} = "PodStopError: no pod was able to receive this file before stopping: ({file})"
}

impl Pod {
    pub async fn new(
        name: String,
        global_config: GlobalConfig,
        local_config: LocalConfig,
        mount_point: WhPath,
        server: Arc<Server>,
        server_address: Address,
    ) -> io::Result<Self> {
        let mut global_config = global_config;

        log::info!("mount point {}", mount_point);
        let (to_network_message_tx, to_network_message_rx) = mpsc::unbounded_channel();
        let (from_network_message_tx, mut from_network_message_rx) = mpsc::unbounded_channel();
        let (to_redundancy_tx, to_redundancy_rx) = mpsc::unbounded_channel();

        global_config.general.peers.retain(|x| *x != server_address);

        let mut peers = vec![];

        let (arbo, next_inode) =
            if let Some((fs_serialized, peers_addrs, ipc, global_config_bytes)) =
                initiate_connection(
                    global_config.general.peers.clone(),
                    server_address.clone(),
                    &from_network_message_tx,
                    &mut from_network_message_rx,
                )
                .await
            {
                if !global_config_bytes.is_empty() {
                    info!(
                        "mount point service: {}",
                        mount_point.join(GLOBAL_CONFIG_FNAME).to_string()
                    );
                    fs::write(
                        mount_point.join(GLOBAL_CONFIG_FNAME).to_string(),
                        global_config_bytes,
                    )
                    .expect("can't write global_config file");
                }
                // TODO use global_config ?

                peers = PeerIPC::peer_startup(peers_addrs, from_network_message_tx.clone()).await;
                peers.push(ipc);
                register_to_others(&peers, &server_address)?;

                let mut arbo = Arbo::new();
                arbo.overwrite_self(fs_serialized.fs_index);

                if let Err(_) = arbo.get_inode(LOCAL_CONFIG_INO) {
                    let _ = arbo.add_inode_from_parameters(
                        LOCAL_CONFIG_FNAME.to_string(),
                        LOCAL_CONFIG_INO,
                        ROOT,
                        FsEntry::File(vec![server_address.clone()]),
                        0o777,
                    );
                }
                let next_inode = arbo.iter().fold(0, |acc, (ino, _)| u64::max(acc, *ino)) + 1;
                (arbo, next_inode)
            } else {
                generate_arbo(&mount_point, &server_address).expect("unable to index folder")
            };

        let redundancy_target = global_config.redundancy.number;
        let arbo: Arc<RwLock<Arbo>> = Arc::new(RwLock::new(arbo));
        let local = Arc::new(RwLock::new(local_config));
        let global = Arc::new(RwLock::new(global_config));

        let network_interface = Arc::new(NetworkInterface::new(
            arbo.clone(),
            mount_point.clone(),
            to_network_message_tx.clone(),
            to_redundancy_tx.clone(),
            next_inode,
            Arc::new(RwLock::new(peers)),
            local.clone(),
            global.clone(),
        ));

        #[cfg(target_os = "linux")]
        let disk_manager = Box::new(UnixDiskManager::new(&mount_point)?);
        #[cfg(target_os = "windows")]
        let disk_manager = Box::new(DummyDiskManager::new(&mount_point)?);

        let fs_interface = Arc::new(FsInterface::new(
            network_interface.clone(),
            disk_manager,
            arbo.clone(),
        ));

        // Start ability to recieve messages
        let network_airport_handle = tokio::spawn(NetworkInterface::network_airport(
            from_network_message_rx,
            fs_interface.clone(),
        ));

        // Start ability to send messages
        let peer_broadcast_handle = tokio::spawn(NetworkInterface::contact_peers(
            network_interface.peers.clone(),
            to_network_message_rx,
        ));

        let new_peer_handle = tokio::spawn(NetworkInterface::incoming_connections_watchdog(
            server,
            from_network_message_tx.clone(),
            network_interface.peers.clone(),
        ));

        let peers = network_interface.peers.clone();

        let redundancy_worker_handle = tokio::spawn(redundancy_worker(
            to_redundancy_rx,
            network_interface.clone(),
            fs_interface.clone(),
            redundancy_target,
            server_address,
        ));

        Ok(Self {
            name: name.clone(),
            network_interface,
            fs_interface: fs_interface.clone(),
            mount_point: mount_point.clone(),
            peers,
            #[cfg(target_os = "linux")]
            fuse_handle: mount_fuse(&mount_point, fs_interface.clone())?,
            #[cfg(target_os = "windows")]
            fsp_host: mount_fsp(&mount_point, fs_interface.clone())?,
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

    pub fn get_file_tree_and_hosts(&self, path: WhPath) -> Result<CliHostTree, PodInfoError> {
        let arbo = Arbo::n_read_lock(&self.network_interface.arbo, "Pod::get_info")?;
        let ino = &arbo
            .get_inode_from_path(&path)
            .map_err(|_| PodInfoError::FileNotFound)?
            .id;

        Ok(CliHostTree {
            lines: Self::recurse_tree(&*arbo, *ino, 0),
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
    fn send_file_to_possible_hosts(
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

            if let Ok(()) = status_rx.blocking_recv().unwrap() {
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
    fn send_files_when_stopping(&self, arbo: &Arbo, peers: Vec<Address>) {
        let address = if let Ok(local_conf_lock) = LocalConfig::read_lock(
            &self.network_interface.local_config,
            "send_files_when_stopping",
        ) {
            local_conf_lock.general.address.clone()
        } else {
            log::error!("send_files_when_stopping: can't lock local conf to get local address. No files sent.");
            return;
        };

        arbo.files_hosted_only_by(&address)
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
            .for_each(|id| {
                if let Err(e) = self.send_file_to_possible_hosts(&peers, id) {
                    log::warn!("{e}");
                }
            });
    }

    pub fn stop(&self) -> Result<(), PodStopError> {
        // TODO
        // in actual state, all operations (request from network other than just pulling the asked files)
        // made after calling this function but before dropping the pod are undefined behavior.

        // drop(self.fuse_handle); // FIXME - do something like block the filesystem

        let arbo = Arbo::n_read_lock(&self.network_interface.arbo, "Pod::Pod::stop(1)")?;

        let peers: Vec<Address> = self
            .peers
            .read()
            .iter()
            .map(|peer| peer.address.clone())
            .collect();

        self.send_files_when_stopping(&arbo, peers);

        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::Disconnect(
                    LocalConfig::read_lock(&self.local_config, "pod::stop")?
                        .general
                        .address
                        .clone(),
                ),
            ))
            .expect("to_network_message_tx closed.");

        let _ = self.fs_interface.disk.remove_file(&ARBO_FILE_FNAME.into());
        self.fs_interface
            .disk
            .write_file(
                &ARBO_FILE_FNAME.into(),
                &bincode::serialize(&*arbo).expect("can't serialize arbo to bincode"),
                0,
            )
            .map(|_| ())
            .map_err(|e| PodStopError::ArboSavingFailed {
                error_source: e.to_string(),
            })?;

        *self.peers.write() = Vec::new(); // dropping PeerIPCs
        self.network_airport_handle.abort();
        self.new_peer_handle.abort();
        self.peer_broadcast_handle.abort();
        Ok(())
    }

    pub fn get_name(&self) -> &str {
        return &self.name;
    }

    pub fn get_mount_point(&self) -> &WhPath {
        return &self.mount_point;
    }
}
