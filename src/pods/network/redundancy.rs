use super::network_interface::{get_all_peers_address, NetworkInterface};
use crate::{
    error::WhResult,
    network::message::RedundancyMessage,
    pods::{arbo::InodeId, filesystem::fs_interface::FsInterface},
};
use std::sync::Arc;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinSet};

/// Redundancy Worker
/// Worker that applies the redundancy to files
pub async fn redundancy_worker(
    mut reception: UnboundedReceiver<RedundancyMessage>,
    nw_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
) {
    loop {
        let message = match reception.recv().await {
            Some(message) => message,
            None => continue,
        };

        match message {
            RedundancyMessage::ApplyTo(ino) => match get_all_peers_address(&nw_interface.peers) {
                Ok(all_peers) => {
                    let file_binary = match fs_interface.read_local_file(ino) {
                        Ok(bin) => bin,
                        Err(e) => {
                            log::error!("Redundancy: can't read file {ino} {e}");
                            continue;
                        }
                    };

                    let target_redundancy = if nw_interface.redundancy as usize > all_peers.len() {
                        log::warn!("Redundancy: Not enough nodes to satisfies the target redundancies number.");
                        all_peers.len()
                    } else {
                        nw_interface.redundancy as usize
                    };

                    push_redundancy(
                        &nw_interface,
                        all_peers,
                        ino,
                        file_binary,
                        target_redundancy,
                    )
                    .await;
                }
                Err(e) => {
                    log::error!("Redundancy: can't add job for {}. Error: {}", ino, e);
                }
            },
        }
    }
}

/// start download to others concurrently
async fn push_redundancy(
    nw_interface: &Arc<NetworkInterface>,
    all_peers: Vec<String>,
    ino: InodeId,
    file_binary: Vec<u8>,
    target_redundancy: usize,
) {
    let mut set: JoinSet<WhResult<()>> = JoinSet::new();
    for i in 0..target_redundancy {
        let nwi_clone = Arc::clone(nw_interface);
        //TODO cloning the whole file content in ram to send it to many hosts is terrible :
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
            Some(Ok(Ok(()))) => (),
            Some(Ok(Err(crate::error::WhError::NetworkDied { called_from: _ }))) => {
                log::warn!("Redundancy: NetworkDied on some host. Trying next...");
                if current_try >= all_peers.len() {
                    log::error!("Redundancy: Not enough answering hosts to apply redundancy.");
                    break;
                }
                let nwi_clone = Arc::clone(nw_interface);
                let bin_clone = file_binary.clone();
                //TODO cloning the whole file content in ram to send it to many hosts is terrible
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
    set.join_all().await;
}
