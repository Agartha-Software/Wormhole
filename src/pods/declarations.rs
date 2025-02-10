use std::{io, sync::Arc};

use log::{debug, info};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{
    fuse::fuse_impl::mount_fuse,
    network::{message::Address, peer_ipc::PeerIPC, server::Server},
};

use super::{
    arbo::{index_folder, Arbo}, disk_manager::DiskManager, fs_interface::FsInterface, network_interface::NetworkInterface, whpath::WhPath
};

// TODO
pub type PodConfig = u64;

pub struct Pod {
    network_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
    mount_point: WhPath,
    peers: Arc<RwLock<Vec<PeerIPC>>>,
    pod_conf: PodConfig,
    fuse_handle: fuser::BackgroundSession,
    network_airport_handle: Option<JoinHandle<()>>,
    peer_broadcast_handle: Option<JoinHandle<()>>,
    new_peer_handle: Option<JoinHandle<()>>,
    background_worker_handle: JoinHandle<()>,
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
        let (to_network_message_tx, to_network_message_rx) = mpsc::unbounded_channel::<BackgroundMission>();
        let (to_network_message_tx, to_network_message_rx) = mpsc::unbounded_channel();
        let (from_network_message_tx, from_network_message_rx) = mpsc::unbounded_channel();
        let (background_mission_tx, background_mission_rx) = mpsc::unbounded_channel();

        let peers = PeerIPC::peer_startup(peers, from_network_message_tx.clone()).await;
        let peers_addrs: Vec<Address> = peers.iter().map(|peer| peer.address.clone()).collect();
        let network_interface = Arc::new(NetworkInterface::new(
            arbo.clone(),
            mount_point.clone(),
            to_network_message_tx,
            next_inode,
            Arc::new(RwLock::new(peers)),
            server_address,
        ));

        let peer_broadcast_handle = Some(tokio::spawn(NetworkInterface::contact_peers(
            network_interface.peers.clone(),
            to_network_message_rx,
        )));

        let disk_manager = DiskManager::new(mount_point.clone())?;

        // TODO - maybe not mount fuse until remote arbo is pulled
        let fs_interface = Arc::new(FsInterface::new(
            network_interface.clone(),
            disk_manager,
            arbo.clone(),
        ));

        let network_airport_handle = Some(tokio::spawn(NetworkInterface::network_airport(
            from_network_message_rx,
            fs_interface.clone(),
        )));

        if peers_addrs.len() >= 1 {
            info!("Will pull filesystem from remote...");
            network_interface
                .request_arbo(peers_addrs[0].clone())
                .await?;

            info!("Pull completed");
            debug!("arbo: {:#?}", network_interface.arbo);
        } else {
            info!("Created fresh new filesystem");
        }

        let new_peer_handle = Some(tokio::spawn(
            NetworkInterface::incoming_connections_watchdog(
                server,
                from_network_message_tx,
                network_interface.peers.clone(),
            ),
        ));

        let peers = network_interface.peers.clone();

        Ok(Self {
            network_interface: network_interface.clone(),
            fs_interface: fs_interface.clone(),
            mount_point: mount_point.clone(),
            peers,
            pod_conf: config,
            fuse_handle: mount_fuse(&mount_point, fs_interface.clone())?,
            network_airport_handle,
            peer_broadcast_handle,
            new_peer_handle,
            background_worker_handle: tokio::spawn(background_worker_airport(
                background_mission_rx,
                fs_interface,
                network_interface,
            )),
        })
    }
}
