use super::network_interface::{get_all_peers_address, NetworkInterface};
use crate::{
    error::{WhError, WhResult},
    network::message::{Address, MessageContent, ToNetworkMessage},
    pods::{
        filesystem::{fs_interface::FsInterface, File},
        itree::{FsEntry, Ino},
    },
};
use core::time;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::SystemTime};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver},
        Semaphore,
    },
    task::JoinSet,
};

custom_error::custom_error! {pub RedundancyError
    WhError{source: WhError} = "{source}",
    InsufficientHosts = "Redundancy: Not enough nodes to satisfies the target redundancies number.", // warning only
}

/// File structs may be kept in ram if
/// smaller than 512KB
const MAX_SIZE_KEEP_RAM: usize = 512 * 1024;

/// Message going to the redundancy worker
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RedundancyMessage {
    ApplyTo(Ino),
    CheckIntegrity,
    UpdatedHosts(Ino, Vec<Address>),
}

#[derive(Clone)]
struct PendingRedundancy {
    pub ino: Ino,
    pub file: Option<File>,
    /// (peer, timeout or tombstone)
    pub pending_sends: Vec<(Address, Option<SystemTime>)>,
    pub hosts: Vec<Address>,
}

impl PendingRedundancy {
    /// Resolves pending sends for addresses
    /// return true if no sends are pending anymore
    pub fn resolve(&mut self, hosts: &[Address]) -> bool {
        self.pending_sends.retain(|(h, _)| !hosts.contains(h));
        self.pending_sends.is_empty()
    }

    /// Create a pending redundancy
    /// tries to send the file so that r_count total peers have it
    /// returns a PendingRedundancy instance
    pub async fn try_once(
        ino: Ino,
        fs_interface: &Arc<FsInterface>,
        all_peers: &[Address],
        r_count: usize,
    ) -> Result<Self, RedundancyError> {
        let timeout = SystemTime::now() + time::Duration::from_secs(10);
        let hosts = match &fs_interface.itree.read_recursive().get_inode(ino)?.entry {
            FsEntry::File(hosts) => Ok(hosts.clone()),
            _ => Err(WhError::InodeIsADirectory),
        }?;
        let mut to = all_peers.to_vec();
        to.retain(|s| !hosts.contains(s));
        let needed = r_count.saturating_sub(hosts.len());
        let file = fs_interface
            .get_local_file(ino)
            .map_err(|e| WhError::WouldBlock {
                called_from: format!("get_local_file: {e}"),
            })
            .and_then(|f| f.ok_or(WhError::InodeIsADirectory))?;
        let sent = push_redundancy(
            &fs_interface.network_interface,
            to,
            ino,
            &file.0.clone(),
            needed,
        )
        .await;
        Ok(Self {
            ino,
            file: (file.0.len() < MAX_SIZE_KEEP_RAM).then_some(file),
            pending_sends: sent
                .iter()
                .map(|t| (t.clone(), Some(timeout)))
                .collect(),
            hosts,
        })
    }

    /// retry sends that have timed out
    /// tries to send to however many have timed out, or less if
    /// someone voluntarily aquired the file outside of the planned process
    pub async fn retry(
        &mut self,
        fs_interface: &Arc<FsInterface>,
        all_peers: &[Address],
        r_count: usize,
    ) -> Result<(), RedundancyError> {
        let now = SystemTime::now();
        let still_pending: usize = self
            .pending_sends
            .iter_mut()
            .map(|(_, t)| t.take_if(|t| *t < now).is_some() as usize)
            .sum();
        let mut remanining_hosts = all_peers.to_vec();
        remanining_hosts.retain(|s| {
            !self.hosts.contains(s) && !self.pending_sends.iter().map(|(p, _)| p).any(|p| p == s)
        });
        let needed = r_count.saturating_sub(self.hosts.len() + still_pending);

        if needed == 0 {
            return Ok(());
        }
        if remanining_hosts.is_empty() {
            return Err(RedundancyError::InsufficientHosts);
        }

        let timeout = SystemTime::now() + time::Duration::from_secs(10);
        let file = match &self.file {
            Some(file) => file.clone(),
            None => fs_interface
                .get_local_file(self.ino)
                .map_err(|e| WhError::WouldBlock {
                    called_from: format!("get_local_file: {e}"),
                })
                .and_then(|f| f.ok_or(WhError::InodeIsADirectory))?,
        };
        let sent = push_redundancy(
            &fs_interface.network_interface,
            remanining_hosts,
            self.ino,
            &file.0,
            needed,
        )
        .await;

        self.pending_sends.append(
            &mut sent
                .iter()
                .map(|t| (t.clone(), Some(timeout)))
                .collect(),
        );
        Ok(())
    }
}

#[derive(Clone, Default)]
struct RedundancyTracker {
    pub pending: Vec<PendingRedundancy>,
}

impl RedundancyTracker {
    /// retry all of the stored pending redundancies for ones that are timed out
    pub async fn retry_timedout(
        &mut self,
        fs_interface: &Arc<FsInterface>,
        all_peers: &[Address],
        r_count: usize,
    ) {
        for pending in self.pending.iter_mut() {
            let _ = pending.retry(fs_interface, all_peers, r_count).await;
        }
    }

    /// check every file in the arbo if it has enough redundancies
    /// then try sending any that are below the quota
    pub async fn full_check(
        &mut self,
        fs_interface: &Arc<FsInterface>,
        all_peers: &[Address],
        r_count: usize,
    ) {
        let needy = fs_interface
            .itree
            .read_recursive()
            .iter()
            .filter_map(|(ino, inode)| match &inode.entry {
                FsEntry::File(hosts) => (hosts.len() < r_count).then_some(*ino),
                _ => None,
            })
            .collect::<Vec<_>>();

        for ino in needy {
            let _ = self.apply(ino, fs_interface, all_peers, r_count).await;
        }
    }

    /// try to send redundancies for a file and track the pending sends
    pub async fn apply(
        &mut self,
        ino: Ino,
        fs_interface: &Arc<FsInterface>,
        all_peers: &[Address],
        r_count: usize,
    ) -> Result<(), RedundancyError> {
        if let Some(pending) = self.pending.iter_mut().find(|p| p.ino == ino) {
            pending
                .retry(fs_interface, all_peers, r_count)
                .await
                .inspect_err(|e| log::error!("Failed to re-apply redundancy to {ino}: {e}"))
        } else {
            PendingRedundancy::try_once(ino, fs_interface, all_peers, r_count)
                .await
                .inspect_err(|e| log::error!("Failed to apply redundancy to {ino}: {e}"))
                .map(|p| {
                    self.pending.push(p);
                })
        }
    }
}

/// Redundancy Worker
/// Worker that applies the redundancy to files
pub async fn redundancy_worker(
    mut reception: UnboundedReceiver<RedundancyMessage>,
    nw_interface: Arc<NetworkInterface>,
    fs_interface: Arc<FsInterface>,
) {
    let mut tracker = RedundancyTracker::default();
    loop {
        let message =
            match tokio::time::timeout(time::Duration::from_secs(1), reception.recv()).await {
                Ok(Some(message)) => message,
                Ok(None) => continue,
                Err(_) => {
                    let all_peers = match get_all_peers_address(&nw_interface.peers) {
                        Ok(peers) => peers,
                        Err(e) => {
                            log::error!("Redundancy: can't get peers: because of: {e}",);
                            continue;
                        }
                    };
                    let r_count = nw_interface.global_config.read().redundancy.number as usize;
                    tracker
                        .retry_timedout(&fs_interface, &all_peers, r_count)
                        .await;
                    continue;
                } // TODO
            };
        let all_peers = match get_all_peers_address(&nw_interface.peers) {
            Ok(peers) => peers,
            Err(e) => {
                log::error!(
                    "Redundancy: can't get peers: (ignoring request \"{:?}\") because of: {e}",
                    message
                );
                continue;
            }
        };
        let r_count = nw_interface.global_config.read().redundancy.number as usize;

        match message {
            RedundancyMessage::ApplyTo(ino) => {
                let _ = tracker.apply(ino, &fs_interface, &all_peers, r_count).await;
            }
            RedundancyMessage::CheckIntegrity => {
                tracker.full_check(&fs_interface, &all_peers, r_count).await;
            }
            RedundancyMessage::UpdatedHosts(ino, hosts) => {
                if let Some(idx) = tracker.pending.iter_mut().position(|p| p.ino == ino) {
                    if tracker.pending[idx].resolve(&hosts) {
                        tracker.pending.swap_remove(idx);
                    }
                }
            }
        };
    }
}

/// start download to others concurrently
/// returns: peers that were sent the file
async fn push_redundancy(
    nw_interface: &Arc<NetworkInterface>,
    to: Vec<Address>,
    ino: Ino,
    file_binary: &Arc<Vec<u8>>,
    target_redundancy: usize,
) -> Vec<Address> {
    let semaphore = Arc::new(Semaphore::new(target_redundancy));
    let remaining = Arc::new(RwLock::<usize>::new(target_redundancy));

    let mut set: JoinSet<Option<Address>> = JoinSet::new();

    for to in to.into_iter() {
        if let Ok(permit) = semaphore.clone().acquire_owned().await {
            let semaphore = semaphore.clone();
            let nwi_clone = nw_interface.clone();
            let bin_clone = file_binary.clone();
            let remaining = remaining.clone();
            set.spawn(async move {
                if let Ok(to) = nwi_clone
                    .send_file_redundancy(ino, bin_clone, to.clone())
                    .await
                {
                    permit.forget();
                    *remaining.write() -= 1;
                    if *remaining.read() == 0 {
                        semaphore.close();
                    }
                    Some(to)
                } else {
                    None
                }
            });
        }
    }
    set.join_all().await.into_iter().flatten().collect()
}

impl NetworkInterface {
    pub async fn send_file_redundancy(
        &self,
        inode: Ino,
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
