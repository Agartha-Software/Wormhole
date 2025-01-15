use std::{io, sync::Arc};

use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{
    fuse::fuse_impl::mount_fuse,
    network::{message::Address, peer_ipc::PeerIPC, server::Server},
};

use super::{
    arbo::{index_folder, Arbo},
    disk_manager::DiskManager,
    fs_interface::FsInterface,
    network_interface::NetworkInterface,
    whpath::WhPath,
};

// TODO
pub type PodConfig = u64;

pub struct Pod {
    network_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
    mount_point: WhPath,
    peers: Vec<PeerIPC>,
    pod_conf: PodConfig,
    fuse_handle: fuser::BackgroundSession,
    network_airport_handle: Option<JoinHandle<()>>,
    peer_broadcast_handle: Option<JoinHandle<()>>,
    new_peer_handle: Option<JoinHandle<()>>,
}

impl Pod {
    pub async fn new(
        mount_point: WhPath,
        config: PodConfig,
        peers: Vec<Address>,
        server: Arc<Server>,
        server_address: Address,
    ) -> io::Result<Self> {
        let (arbo, next_inode) = index_folder(&mount_point)?;
        let arbo: Arc<RwLock<Arbo>> = Arc::new(RwLock::new(arbo));
        let (to_network_message_tx, to_network_message_rx) = mpsc::unbounded_channel();
        let (from_network_message_tx, from_network_message_rx) = mpsc::unbounded_channel();

        let network_interface = Arc::new(NetworkInterface::new(
            arbo.clone(),
            mount_point.clone(),
            to_network_message_tx,
            next_inode,
            server_address
        ));

        let disk_manager = DiskManager::new(mount_point.clone())?;

        let fs_interface = Arc::new(FsInterface::new(
            network_interface.clone(),
            disk_manager,
            arbo.clone(),
        ));

        let network_airport_handle = Some(tokio::spawn(NetworkInterface::network_airport(
            from_network_message_rx,
            fs_interface.clone(),
        )));
        let peer_broadcast_handle = Some(tokio::spawn(NetworkInterface::contact_peers(
            network_interface.peers.clone(),
            to_network_message_rx,
        )));
        let new_peer_handle = Some(tokio::spawn(NetworkInterface::incoming_connections_watchdog(
            server,
            from_network_message_tx.clone(),
            network_interface.peers.clone(),
        )));

        Ok(Self {
            network_interface,
            fs_interface: fs_interface.clone(),
            mount_point: mount_point.clone(),
            peers: PeerIPC::peer_startup(peers, from_network_message_tx).await,
            pod_conf: config,
            fuse_handle: mount_fuse(&mount_point, fs_interface.clone())?,
            network_airport_handle,
            peer_broadcast_handle,
            new_peer_handle,
        })
    }
}
