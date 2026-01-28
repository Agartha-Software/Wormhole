use super::network_interface::NetworkInterface;
use crate::{
    error::WhError,
    network::message::{Request, ToNetworkMessage},
    pods::{
        filesystem::{fs_interface::FsInterface, File},
        itree::{FsEntry, ITree, Ino},
    },
};
use either::Either;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{
    sync::{mpsc::UnboundedReceiver, oneshot},
    task::{AbortHandle, JoinSet},
};

custom_error::custom_error! {pub RedundancyError
    WhError{source: WhError} = "{source}",
    InsufficientHosts = "Redundancy: Not enough nodes to satisfies the target redundancies number.", // warning only
    IsLocalOnly = "Redundancy: this inode is set to not replicate.", // warning only
}

/// File structs may be kept in ram if
/// smaller than 512KB
const MAX_SIZE_KEEP_RAM: usize = 512 * 1024;

/// Message going to the redundancy worker
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RedundancyMessage {
    ApplyTo(Ino),
    CheckIntegrity,
}

type Tombstone = ();

/// Left<Tombstone> if the send failed, Right<AbortHandle> if it's still pending
type PendingStatus = Either<Tombstone, AbortHandle>;
#[derive(Clone)]
struct PendingRedundancy {
    pub ino: Ino,
    pub file: Option<File>,
    // pub sends: Vec<PeerId>,
    pub sends: Vec<(PeerId, PendingStatus)>,
    // pub hosts: Vec<PeerId>,
}

/// Tracks files that are being sent
/// enables sending files again if a peer fails to receive
struct RedundancyTracker {
    /// asychronous sending tasks, waiting on the reply from the peer
    pub tasks: JoinSet<Result<(Ino, PeerId), Ino>>,
    /// metadata about each file's pending status
    pub pending: Vec<PendingRedundancy>,
    /// FsInterface for convenience
    pub fs_interface: Arc<FsInterface>,
}

impl RedundancyTracker {
    pub fn new(fs_interface: Arc<FsInterface>) -> Self {
        Self {
            fs_interface,
            tasks: Default::default(),
            pending: Default::default(),
        }
    }

    /// check every file in the arbo if it has enough redundancies
    /// then try sending any that are below the quota
    pub async fn full_check(
        &mut self,
        fs_interface: &Arc<FsInterface>,
        all_peers: &[PeerId],
        r_count: usize,
    ) {
        let needy = fs_interface
            .network_interface
            .itree
            .read()
            .iter()
            .filter_map(|(ino, inode)| match &inode.entry {
                FsEntry::File(hosts) => {
                    (hosts.len() < r_count && !ITree::is_local_only(*ino)).then_some(*ino)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        for ino in needy {
            let _ = self.apply(ino, all_peers, r_count).await;
        }
    }

    /// try to send redundancies for a file and track the pending sends
    /// does nothing if the file is already trying to sync
    pub async fn apply(
        &mut self,
        ino: Ino,
        all_peers: &[PeerId],
        r_count: usize,
    ) -> Result<(), RedundancyError> {
        if self.pending.iter_mut().any(|p| p.ino == ino) {
            return Ok(());
        }
        self.try_once(ino, all_peers, r_count)
            .await
            // .map(|p| {
            //     self.pending.push(p);
            // })
            .or_else(|e| {
                matches!(e, RedundancyError::IsLocalOnly)
                    .then_some(())
                    .ok_or(e)
            })
            .inspect_err(|e| log::error!("Failed to apply redundancy to {ino}: {e}"))
    }

    /// try to send redundancies for a file and track the pending sends
    /// fails if the file is local only
    pub async fn try_once(
        &mut self,
        ino: Ino,
        peers: &[PeerId],
        r_count: usize,
    ) -> Result<(), RedundancyError> {
        if ITree::is_local_only(ino) {
            return Err(RedundancyError::IsLocalOnly);
        }
        let hosts = self
            .fs_interface
            .network_interface
            .itree
            .read_recursive()
            .get_inode_hosts(ino)?
            .to_vec();

        if !hosts.contains(&self.fs_interface.network_interface.id) {
            return Ok(());
        }

        let mut to = peers.to_vec();
        to.retain(|s| !hosts.contains(s));

        let needed = r_count.saturating_sub(hosts.len());
        let file = self
            .fs_interface
            .get_local_file(ino)
            .map_err(|e| WhError::WouldBlock {
                called_from: format!("get_local_file: {e}"),
            })
            .and_then(|f| f.ok_or(WhError::InodeNotFound))?;
        let sends = Self::push_redundancy(
            &self.fs_interface,
            &mut self.tasks,
            &to,
            ino,
            &file.0.clone(),
            needed,
        )
        .await;
        self.pending.push(PendingRedundancy {
            ino,
            file: (file.0.len() < MAX_SIZE_KEEP_RAM).then_some(file),
            sends,
            // hosts,
        });
        Ok(())
    }

    /// resolve a pending stored send
    /// removes the file from the tracker if all pending send are resolved
    pub fn resolve(&mut self, ino: Ino, peer: PeerId) {
        let remove = if let Some(p_index) = self.pending.iter().position(|p| p.ino == ino) {
            let pending = &mut self.pending[p_index];
            if let Some(s_index) = pending.sends.iter().position(|s| s.0 == peer) {
                pending.sends.swap_remove(s_index);
            }
            if pending.sends.iter().all(|(_, status)| status.is_left()) {
                Some(p_index)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(p_index) = remove {
            self.pending.swap_remove(p_index);
        }
    }

    /// retry sending the file to the next available peer.
    /// skips peers that have the file and peers that previously have failed
    pub async fn retry(
        &mut self,
        ino: Ino,
        all_peers: &[PeerId],
        r_count: usize,
    ) -> Result<(), RedundancyError> {
        if let Some(pending) = self.pending.iter_mut().find(|p| p.ino == ino) {
            let hosts = self
                .fs_interface
                .network_interface
                .itree
                .read()
                .get_inode_hosts(ino)?
                .to_vec();
            let mut remanining_hosts = all_peers.to_vec();
            remanining_hosts.retain(|host| {
                !hosts.contains(host) && !pending.sends.iter().any(|s| s.0 == *host)
            });
            let needed = r_count.saturating_sub(hosts.len());

            if needed == 0 {
                return Ok(());
            }
            if remanining_hosts.is_empty() {
                return Err(RedundancyError::InsufficientHosts);
            }

            let file = match &pending.file {
                Some(file) => file.clone(),
                None => self
                    .fs_interface
                    .get_local_file(ino)
                    .map_err(|e| WhError::WouldBlock {
                        called_from: format!("get_local_file: {e}"),
                    })
                    .and_then(|f| f.ok_or(WhError::InodeIsADirectory))?,
            };
            let mut sent = Self::push_redundancy(
                &self.fs_interface,
                &mut self.tasks,
                &remanining_hosts,
                ino,
                &file.0,
                needed,
            )
            .await;

            pending.sends.append(&mut sent);
        }
        Ok(())
    }

    /// remove a file from the tracker, regardless of if it's done processing
    /// meant to be used when there are no remaining hosts to send to, so no point tracking
    pub fn forget(&mut self, ino: Ino) {
        if let Some(index) = self.pending.iter().position(|p| p.ino == ino) {
            self.pending.swap_remove(index);
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
    let mut tracker = RedundancyTracker::new(fs_interface.clone());
    loop {
        tokio::select! {
            resolve = tracker.tasks.join_next(), if !tracker.tasks.is_empty()  => {
                match resolve {
                    Some(Ok(Ok((ino, peer)))) => tracker.resolve(ino, peer),
                    Some(Ok(Err(ino))) => {
                        let r_count = nw_interface.global_config.read().redundancy.number as usize;
                        let peers = nw_interface.peers.read().clone();
                        if let Err(RedundancyError::InsufficientHosts) = tracker.retry(ino, &peers, r_count).await {
                            tracker.forget(ino)
                        }
                    }
                    Some(Err(_)) => {},
                    None => {},
                }
            },
            message = reception.recv() => {
                let r_count = nw_interface.global_config.read().redundancy.number as usize;
                let peers = nw_interface.peers.read().clone();

                match message {
                    Some(RedundancyMessage::ApplyTo(ino)) => {
                        let _ = tracker.apply(ino, &peers, r_count).await;
                    }
                    Some(RedundancyMessage::CheckIntegrity) => {
                        tracker.full_check(&fs_interface, &peers, r_count).await;
                    }
                    None => { return }
                }
            }
        };
    }
}

impl RedundancyTracker {
    /// start download to others concurrently
    async fn push_redundancy(
        fs_interface: &FsInterface,
        tasks: &mut JoinSet<Result<(Ino, PeerId), Ino>>,
        to: &[PeerId],
        ino: Ino,
        file_binary: &Arc<Vec<u8>>,
        target_redundancy: usize,
    ) -> Vec<(PeerId, PendingStatus)> {
        let mut workers = Vec::new();

        for to in to.iter().copied().take(target_redundancy) {
            let nwi_clone = fs_interface.network_interface.clone();
            let bin_clone = file_binary.clone();
            let handle = tasks
                .spawn(async move { nwi_clone.send_file_redundancy(ino, bin_clone, to).await });
            workers.push((to, Either::Right(handle)));
        }
        workers
    }
}

impl NetworkInterface {
    /// send a file redundancy to a peer
    pub async fn send_file_redundancy(
        &self,
        ino: Ino,
        data: Arc<Vec<u8>>,
        to: PeerId,
    ) -> Result<(Ino, PeerId), Ino> {
        let (status_tx, status_rx) = oneshot::channel();

        self.to_network_message_tx
            .send(ToNetworkMessage::AnswerMessage(
                Request::RedundancyFile(ino, data),
                status_tx,
                to,
            ))
            .expect("send_file: unable to update modification on the network thread");

        status_rx.await.ok().flatten().map(|_| (ino, to)).ok_or(ino)
    }
}
