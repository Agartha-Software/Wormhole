use super::network_interface::NetworkInterface;
use crate::{
    error::{WhError, WhResult},
    network::message::{RedundancyMessage, Request, ToNetworkMessage},
    pods::{
        filesystem::fs_interface::FsInterface,
        itree::{FsEntry, ITree, Ino},
    },
};
use futures_util::future::join_all;
use libp2p::PeerId;
use std::sync::Arc;
use tokio::{
    sync::{mpsc::UnboundedReceiver, oneshot},
    task::JoinSet,
};

custom_error::custom_error! {pub RedundancyError
    WhError{source: WhError} = "{source}",
    InsufficientHosts = "Redundancy: Not enough nodes to satisfies the target redundancies number.", // warning only
}

/// Redundancy Worker
/// Worker that applies the redundancy to files
pub async fn redundancy_worker(
    mut reception: UnboundedReceiver<RedundancyMessage>,
    nw_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
    // redundancy: u64, // TODO - when updated in conf, send a message to this worker for update
) {
    loop {
        let message = match reception.recv().await {
            Some(message) => message,
            None => return,
        };
        let peers = nw_interface.peers.read().clone();

        match message {
            RedundancyMessage::ApplyTo(ino) => {
                let _ = apply_to(&nw_interface, &fs_interface, &peers, ino)
                    .await
                    .inspect_err(|e| log::error!("Redundancy error: {e}"));
            }
            RedundancyMessage::CheckIntegrity => {
                let _ = check_integrity(&nw_interface, &fs_interface, &peers)
                    .await
                    .inspect_err(|e| log::error!("Redundancy error: {e}"));
            }
        };
    }
}

/// Checks if an inode can have it's redundancy applied :
/// - needs more hosts
/// - the network contains more hosts
/// - this node possesses the file
/// - this node is first on the sorted hosts list (naive approach to avoid many hosts applying the same file)
///
/// Intended for use in the check_intergrity function
fn eligible_to_apply(
    ino: Ino,
    entry: &FsEntry,
    target_redundancy: u64,
    available_peers: usize,
    self_addr: &PeerId,
) -> Option<Ino> {
    if ITree::is_local_only(ino) {
        return None;
    }
    let hosts = if let FsEntry::File(hosts) = entry {
        let mut hosts = hosts.clone();
        hosts.sort();
        hosts
    } else {
        return None;
    };
    if hosts.len() < target_redundancy as usize
        && available_peers > hosts.len()
        && hosts.first() == Some(self_addr)
    {
        Some(ino)
    } else {
        None
    }
}

async fn check_integrity(
    nw_interface: &Arc<NetworkInterface>,
    fs_interface: &Arc<FsInterface>,
    peers: &[PeerId],
) -> WhResult<()> {
    let available_peers = peers.len() + 1;

    // Applies redundancy to needed files
    let selected_files: Vec<Ino> =
        ITree::read_lock(&nw_interface.itree, "redundancy: check_integrity")?
            .iter()
            .filter_map(|(ino, inode)| {
                eligible_to_apply(
                    *ino,
                    &inode.entry,
                    nw_interface.global_config.read().redundancy.number,
                    available_peers,
                    &nw_interface.id,
                )
            })
            .collect();
    let futures = selected_files
        .iter()
        .map(|ino| apply_to(nw_interface, fs_interface, peers, *ino))
        .collect::<Vec<_>>();

    let errors: Vec<WhError> = join_all(futures)
        .await
        .into_iter()
        .filter_map(Result::err)
        .collect();

    if !errors.is_empty() {
        log::error!(
            "Redundancy::check_integrity: {} errors reported ! See below:",
            errors.len()
        );
        errors.iter().for_each(|e| log::error!("{e}"));
    }
    Ok(())
}

async fn apply_to(
    nw_interface: &Arc<NetworkInterface>,
    fs_interface: &Arc<FsInterface>,
    peers: &[PeerId],
    ino: u64,
) -> WhResult<usize> {
    if ITree::is_local_only(ino) {
        return Ok(0);
    }
    let redundancy = nw_interface.global_config.read().redundancy.number;

    if redundancy == 0 {
        return Ok(0);
    }

    let file_binary = Arc::new(fs_interface.read_local_file(ino)?);

    let missing_hosts_count: usize;
    let available_hosts = peers.len() + 1; // + myself
    let target_redundancy = if redundancy as usize > available_hosts {
        missing_hosts_count = redundancy as usize - peers.len();
        peers.len()
    } else {
        missing_hosts_count = 0;
        (redundancy - 1) as usize
    };

    let new_hosts = push_redundancy(nw_interface, peers, ino, file_binary, target_redundancy).await;

    nw_interface.update_hosts(ino, new_hosts)?;
    Ok(missing_hosts_count)
}

/// start download to others concurrently
async fn push_redundancy(
    nw_interface: &Arc<NetworkInterface>,
    all_peers: &[PeerId],
    ino: Ino,
    file_binary: Arc<Vec<u8>>,
    target_redundancy: usize,
) -> Vec<PeerId> {
    let mut success_hosts: Vec<PeerId> = vec![nw_interface.id];
    let mut set: JoinSet<Option<PeerId>> = JoinSet::new();

    for addr in all_peers.iter().take(target_redundancy).cloned() {
        let nwi_clone = Arc::clone(nw_interface);
        let bin_clone = file_binary.clone();

        set.spawn(async move { nwi_clone.send_file_redundancy(ino, bin_clone, addr).await });
    }

    // check for success and try next hosts if failure
    let mut current_try = target_redundancy;
    loop {
        match set.join_next().await {
            None => break,
            Some(Err(e)) => {
                log::error!("redundancy_worker: error in thread pool: {e}");
                break;
            }
            Some(Ok(Some(host))) => success_hosts.push(host),
            Some(Ok(None)) => {
                log::warn!("Redundancy: NetworkDied on some host. Trying next...");
                if current_try >= all_peers.len() {
                    log::error!("Redundancy: Not enough answering hosts to apply redundancy.");
                    break;
                }
                let nwi_clone = Arc::clone(nw_interface);
                let bin_clone = file_binary.clone();
                let addr = all_peers[current_try];

                set.spawn(
                    async move { nwi_clone.send_file_redundancy(ino, bin_clone, addr).await },
                );
                current_try += 1;
            }
        }
    }
    success_hosts
}

impl NetworkInterface {
    pub async fn send_file_redundancy(
        &self,
        inode: Ino,
        data: Arc<Vec<u8>>,
        to: PeerId,
    ) -> Option<PeerId> {
        let (status_tx, status_rx) = oneshot::channel();

        self.to_network_message_tx
            .send(ToNetworkMessage::AnswerMessage(
                Request::RedundancyFile(inode, data),
                status_tx,
                to,
            ))
            .expect("send_file: unable to update modification on the network thread");

        status_rx.await.ok().flatten().map(|_| to)
    }
}
