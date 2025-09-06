use super::network_interface::{get_all_peers_address, NetworkInterface};
use crate::{
    error::{WhError, WhResult},
    network::message::{Address, MessageContent, RedundancyMessage, ToNetworkMessage},
    pods::{
        arbo::{Arbo, FsEntry, Ino, InodeId},
        filesystem::fs_interface::FsInterface,
    },
};
use futures_util::future::join_all;
use std::sync::Arc;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
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
            None => continue,
        };
        let peers = match get_all_peers_address(&nw_interface.peers) {
            Ok(peers) => peers,
            Err(e) => {
                log::error!(
                    "Redundancy: can't get peers: (ignoring request \"{:?}\") because of: {e}",
                    message
                );
                continue;
            }
        };

        let _ = match message {
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
    ino: InodeId,
    entry: &FsEntry,
    target_redundancy: u64,
    available_peers: usize,
    self_addr: &Address,
) -> Option<InodeId> {
    if Arbo::is_local_only(ino) {
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
        && hosts[0] == *self_addr
    {
        Some(ino)
    } else {
        None
    }
}

async fn check_integrity(
    nw_interface: &Arc<NetworkInterface>,
    fs_interface: &Arc<FsInterface>,
    peers: &Vec<Address>,
) -> WhResult<()> {
    let available_peers = peers.len() + 1;

    let hostname = nw_interface.hostname()?;

    // Applies redundancy to needed files
    let selected_files: Vec<Ino> =
        Arbo::n_read_lock(&nw_interface.arbo, "redundancy: check_integrity")?
            .iter()
            .filter_map(|(ino, inode)| {
                eligible_to_apply(
                    *ino,
                    &inode.entry,
                    nw_interface.global_config.read().redundancy.number,
                    available_peers,
                    &hostname,
                )
            })
            .collect();
    let futures = selected_files
        .iter()
        .map(|ino| apply_to(nw_interface, fs_interface, peers, ino.clone()))
        .collect::<Vec<_>>();

    let errors: Vec<WhError> = join_all(futures)
        .await
        .into_iter()
        .filter_map(|status| match status {
            Err(e) => Some(e),
            _ => None,
        })
        .collect();

    if errors.len() > 0 {
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
    peers: &Vec<Address>,
    ino: u64,
) -> WhResult<usize> {
    if Arbo::is_local_only(ino) {
        return Ok(0);
    }
    let file_binary = Arc::new(fs_interface.read_local_file(ino)?);

    let redundancy = nw_interface.global_config.read().redundancy.number;

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
    all_peers: &Vec<String>,
    ino: InodeId,
    file_binary: Arc<Vec<u8>>,
    target_redundancy: usize,
) -> Vec<Address> {
    let mut success_hosts: Vec<Address> = vec![nw_interface.hostname().unwrap()];
    let mut set: JoinSet<WhResult<Address>> = JoinSet::new();

    for i in 0..target_redundancy {
        let nwi_clone = Arc::clone(nw_interface);
        let bin_clone = file_binary.clone();
        let addr = all_peers[i].clone();

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
            Some(Ok(Ok(host))) => success_hosts.push(host),
            Some(Ok(Err(crate::error::WhError::NetworkDied { called_from: _ }))) => {
                log::warn!("Redundancy: NetworkDied on some host. Trying next...");
                if current_try >= all_peers.len() {
                    log::error!("Redundancy: Not enough answering hosts to apply redundancy.");
                    break;
                }
                let nwi_clone = Arc::clone(nw_interface);
                let bin_clone = file_binary.clone();
                let addr = all_peers[current_try].clone();

                set.spawn(
                    async move { nwi_clone.send_file_redundancy(ino, bin_clone, addr).await },
                );
                current_try += 1;
            }
            Some(Ok(Err(e))) => {
                log::error!("Redundancy: unknown error when applying redundancy: {e}");
                break;
            }
        }
    }
    success_hosts
}

impl NetworkInterface {
    pub async fn send_file_redundancy(
        &self,
        inode: InodeId,
        data: Arc<Vec<u8>>,
        to: Address,
    ) -> WhResult<Address> {
        let (status_tx, mut status_rx) = unbounded_channel();

        self.to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                (MessageContent::RedundancyFile(inode, data), Some(status_tx)),
                vec![to.clone()],
            ))
            .expect("send_file: unable to update modification on the network thread");

        status_rx
            .recv()
            .await
            .unwrap_or(Err(WhError::NetworkDied {
                called_from: "network_interface::send_file_redundancy".to_owned(),
            }))
            .map(|()| to)
    }
}
