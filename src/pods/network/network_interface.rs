use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::UNIX_EPOCH,
};

use crate::{
    config::GlobalConfig,
    error::{WhError, WhResult},
    ipc::answers::PeerInfo,
    network::message::{Request, Response, ToNetworkMessage},
    pods::{
        filesystem::make_inode::MakeInodeError, network::redundancy::RedundancyMessage,
        whpath::InodeName,
    },
};
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::pods::filesystem::{remove_inode::RemoveInodeError, rename::RenameError};
use crate::pods::itree::{FsEntry, Metadata};

use crate::pods::itree::{ITree, Ino, Inode, LOCK_TIMEOUT};

pub struct NetworkInterface {
    pub itree: Arc<RwLock<ITree>>,
    pub id: PeerId,
    pub to_network_message_tx: UnboundedSender<ToNetworkMessage>,
    pub to_redundancy_tx: UnboundedSender<RedundancyMessage>,
    pub global_config: Arc<RwLock<GlobalConfig>>,
    pub listen_addrs: Arc<RwLock<HashSet<Multiaddr>>>,
    pub peers: Arc<RwLock<Vec<PeerId>>>,
    pub peers_info: Arc<RwLock<HashMap<PeerId, PeerInfo>>>, // Only used to store state for restart and inspect
}

impl NetworkInterface {
    pub fn new(
        itree: Arc<RwLock<ITree>>,
        id: PeerId,
        to_network_message_tx: UnboundedSender<ToNetworkMessage>,
        to_redundancy_tx: UnboundedSender<RedundancyMessage>,
        peers: Arc<RwLock<Vec<PeerId>>>,
        global_config: Arc<RwLock<GlobalConfig>>,
    ) -> Self {
        Self {
            itree,
            id,
            to_network_message_tx,
            to_redundancy_tx,
            peers,
            global_config,
            listen_addrs: Arc::new(RwLock::new(HashSet::new())),
            peers_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add the requested entry to the itree and inform the network
    pub fn register_new_inode(&self, inode: Inode) -> Result<(), MakeInodeError> {
        ITree::write_lock(&self.itree, "register_new_inode")?.add_inode(inode.clone())?;

        if !ITree::is_local_only(inode.id) {
            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(Request::Inode(inode)))
                .expect("register inode: unable to update modification on the network thread");
        }
        Ok(())
        // TODO - if unable to update for some reason, should be passed to the background worker
    }

    pub fn rename(
        &self,
        parent: Ino,
        new_parent: Ino,
        name: InodeName,
        new_name: InodeName,
        overwrite: bool,
    ) -> Result<(), RenameError> {
        let mut itree = ITree::write_lock(&self.itree, "itree_rename_file")?;

        itree.mv_inode(parent, new_parent, name.as_ref(), new_name.clone())?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(Request::Rename(
                parent, new_parent, name, new_name, overwrite,
            )))
            .expect("broadcast_rename_file: unable to update modification on the network thread");
        Ok(())
    }

    pub fn acknowledge_rename(
        &self,
        parent: Ino,
        new_parent: Ino,
        name: InodeName,
        new_name: InodeName,
    ) -> Result<(), RenameError> {
        let mut itree = ITree::write_lock(&self.itree, "itree_rename_file")?;

        itree
            .mv_inode(parent, new_parent, name.as_ref(), new_name)
            .map_err(|err| match err {
                WhError::InodeNotFound => RenameError::DestinationParentNotFound,
                WhError::InodeIsNotADirectory => RenameError::DestinationParentNotFolder,
                source => RenameError::WhError { source },
            })
    }

    /// Get a new inode, add the requested entry to the itree and inform the network
    /// marks as reserved the Ino range up to the new Inode id
    pub fn acknowledge_new_file(&self, inode: Inode) -> Result<(), MakeInodeError> {
        let mut itree = ITree::write_lock(&self.itree, "acknowledge_new_file")?;
        let _ = itree.mark_reserved_ino(inode.id); // this only happens in out-of-order handling of peer's inode creation, and isn't really an error
        itree.add_inode(inode)
    }

    /// Remove [Inode] from the [ITree] and inform the network of the removal
    pub fn unregister_inode(&self, id: Ino) -> Result<(), RemoveInodeError> {
        ITree::write_lock(&self.itree, "unregister_inode")?.remove_inode(id)?;

        if !ITree::is_local_only(id) {
            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(Request::Remove(id)))
                .expect("unregister_inode: unable to update modification on the network thread");
        }
        // TODO - if unable to update for some reason, should be passed to the background worker
        Ok(())
    }

    /// Remove [Inode] from the [ITree]
    pub fn acknowledge_unregister_inode(&self, id: Ino) -> Result<Inode, RemoveInodeError> {
        ITree::write_lock(&self.itree, "acknowledge_unregister_inode")?.remove_inode(id)
    }

    pub fn add_inode_hosts(&self, ino: Ino, hosts: Vec<PeerId>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "network_interface::update_hosts")?
            .add_inode_hosts(ino, hosts.clone())?;

        if !ITree::is_local_only(ino) {
            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(Request::AddHosts(
                    ino, hosts,
                )))
                .expect("update_remote_hosts: unable to update modification on the network thread");
        }
        Ok(())
    }

    pub fn aknowledge_new_hosts(&self, id: Ino, new_hosts: Vec<PeerId>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "aknowledge_new_hosts")?.add_inode_hosts(id, new_hosts)
    }

    pub fn aknowledge_hosts_removal(&self, id: Ino, new_hosts: Vec<PeerId>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "aknowledge_hosts_removal")?
            .remove_inode_hosts(id, new_hosts)
    }

    pub fn update_metadata(&self, id: Ino, meta: Metadata) -> WhResult<()> {
        let mut itree = ITree::write_lock(&self.itree, "network_interface::update_metadata")?;
        let mut fixed_meta = meta;
        let ref_meta = &itree.get_inode(id)?.meta;

        // meta's SystemTime is fragile: it can be silently corrupted such that
        // serialization leads to a failure we can't deal with
        if fixed_meta.atime.duration_since(UNIX_EPOCH).is_err() {
            fixed_meta.atime = ref_meta.atime;
        }

        if fixed_meta.ctime.duration_since(UNIX_EPOCH).is_err() {
            fixed_meta.ctime = ref_meta.ctime;
        }

        if fixed_meta.crtime.duration_since(UNIX_EPOCH).is_err() {
            fixed_meta.crtime = ref_meta.crtime;
        }

        if fixed_meta.mtime.duration_since(UNIX_EPOCH).is_err() {
            fixed_meta.mtime = ref_meta.mtime;
        }

        itree.set_inode_meta(id, fixed_meta.clone())?;
        drop(itree);

        if !ITree::is_local_only(id) {
            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(Request::EditMetadata(
                    id, fixed_meta,
                )))
                .expect("update_metadata: unable to update modification on the network thread");
        }
        Ok(())
        /* REVIEW
         * This system (and others broadcasts systems) should be reviewed as they don't check success.
         * In this case, if another host misses this order, it will not update it's file.
         * We could create a "broadcast" callback with the number of awaited confirmations and a timeout
         * before resend or fail declaration.
         * Or send a bunch of Specific messages
         */
    }

    // SECTION Redundancy related

    pub fn apply_redundancy(&self, file_id: Ino) {
        self.to_redundancy_tx
            .send(RedundancyMessage::ApplyTo(file_id))
            .expect("network_interface::apply_redundancy: tx error");
    }

    pub fn check_integrity(&self) {
        self.to_redundancy_tx
            .send(RedundancyMessage::CheckIntegrity)
            .expect("network_interface::apply_redundancy: tx error");
    }

    // !SECTION ^ Redundancy related

    // SECTION Node related

    // pub fn register_to_others(&self) {
    //     self.to_network_message_tx
    //         .send(ToNetworkMessage::BroadcastMessage(
    //             Request::Register(
    //                 LocalConfig::read_lock(
    //                     &self.local_config,
    //                     ".",
    //                 )
    //                 .expect("network_interface::register_to_others: can't read the address in the local config")
    //                 .general
    //                 .address
    //                 .clone(),
    //             ),
    //         ))
    //         .expect("register_to_others: unable to update modification on the network thread");
    // }

    // pub fn edit_peer_ip(&self, actual: Address, new: Address) {
    //     log::info!("changing host {} to {}", actual, new);
    //     if let Some(mut peers) = self.peers.try_write_for(LOCK_TIMEOUT) {
    //         for peer in peers.iter_mut() {
    //             if peer.address == actual {
    //                 log::info!("done once");
    //                 peer.address = new.clone();
    //             }
    //         }
    //     }
    // }

    pub fn send_filesystem(&self, to: PeerId) -> WhResult<Response> {
        let clean_itree = self.itree.read().clone().clean_local();

        let mut peers_address_list = self.peers_info.read().clone();
        peers_address_list.remove(&to);

        log::trace!("send fs; {peers_address_list:?}");

        let global_config = self.global_config.read().clone();

        Ok(Response::FsAnswer(
            clean_itree,
            peers_address_list,
            global_config,
        ))
    }

    pub fn disconnect_peer(&self, addr: PeerId) -> WhResult<Response> {
        self.peers
            .try_write_for(LOCK_TIMEOUT)
            .ok_or(WhError::WouldBlock {
                called_from: "disconnect_peer: can't write lock peers".to_owned(),
            })?
            .retain(|p| p != &addr);

        log::debug!("Disconnecting {addr}. Removing from inodes hosts");
        for inode in ITree::write_lock(&self.itree, "disconnect_peer")?.inodes_mut() {
            if let FsEntry::File(hosts) = &mut inode.entry {
                hosts.retain(|h| *h != addr);
            }
        }
        self.check_integrity();
        Ok(Response::Success)
    }
}
