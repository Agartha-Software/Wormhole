use std::{
    io::{self, ErrorKind},
    sync::Arc,
    time::UNIX_EPOCH,
};

use crate::{
    config::{GlobalConfig, LocalConfig},
    error::{WhError, WhResult},
    network::{
        message::{
            Address, FileSystemSerialized, FromNetworkMessage, MessageAndStatus, MessageContent,
            RedundancyMessage, ToNetworkMessage,
        },
        peer_ipc::PeerIPC,
        server::Server,
    },
    pods::{filesystem::make_inode::MakeInodeError, itree::Ino, whpath::InodeName},
};
use parking_lot::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

use crate::pods::filesystem::{remove_inode::RemoveInodeError, rename::RenameError};
use crate::pods::itree::{FsEntry, Metadata};

use crate::pods::{
    filesystem::fs_interface::FsInterface,
    itree::{ITree, Inode, InodeId, LOCK_TIMEOUT},
};

use crate::pods::network::callbacks::Callbacks;

// We use a function here because we need templates, but we don't want to leak this kind of weird function to anywhere else
fn into_boxed_io<T: std::error::Error>(err: T) -> io::Error {
    std::io::Error::other(format!("{}: {err}", std::any::type_name::<T>()))
}

pub fn get_all_peers_address(peers: &Arc<RwLock<Vec<PeerIPC>>>) -> WhResult<Vec<String>> {
    Ok(peers
        .try_read_for(LOCK_TIMEOUT)
        .ok_or(WhError::WouldBlock {
            called_from: "get_all_peers_address: can't lock peers mutex".to_string(),
        })?
        .iter()
        .map(|peer| peer.hostname.clone())
        .collect::<Vec<String>>())
}
#[derive(Debug)]
pub struct NetworkInterface {
    pub itree: Arc<RwLock<ITree>>,
    pub url: Option<String>,
    pub to_network_message_tx: UnboundedSender<ToNetworkMessage>,
    pub to_redundancy_tx: UnboundedSender<RedundancyMessage>,
    pub callbacks: Callbacks,
    pub peers: Arc<RwLock<Vec<PeerIPC>>>,
    pub local_config: Arc<RwLock<LocalConfig>>,
    pub global_config: Arc<RwLock<GlobalConfig>>,
}

impl NetworkInterface {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        itree: Arc<RwLock<ITree>>,
        url: Option<String>,
        to_network_message_tx: UnboundedSender<ToNetworkMessage>,
        to_redundancy_tx: UnboundedSender<RedundancyMessage>,
        peers: Arc<RwLock<Vec<PeerIPC>>>,
        local_config: Arc<RwLock<LocalConfig>>,
        global_config: Arc<RwLock<GlobalConfig>>,
    ) -> Self {
        Self {
            itree,
            url,
            to_network_message_tx,
            to_redundancy_tx,
            callbacks: Callbacks::new(),
            peers,
            local_config,
            global_config,
        }
    }

    pub fn hostname(&self) -> WhResult<String> {
        Ok(self
            .local_config
            .try_read()
            .ok_or(WhError::DeadLock)?
            .general
            .hostname
            .clone())
    }

    /** TODO: Doc when reviews are finished */
    #[deprecated(note = "bad to preallocate inodes like this")]
    pub fn get_next_inode(&self) -> WhResult<u64> {
        self.itree
            .write()
            .next_ino
            .next()
            .ok_or(WhError::WouldBlock {
                called_from: "get_next_inode".to_owned(),
            })
            .inspect_err(|_| log::error!("Ran out of Ino, returning Wh::WouldBlock"))

        // let mut next_inode =
        //     self.next_inode
        //         .try_lock_for(LOCK_TIMEOUT)
        //         .ok_or(WhError::WouldBlock {
        //             called_from: "get_next_inode".to_string(),
        //         })?;

        // let available_inode = *next_inode;
        // *next_inode += 1;

        // Ok(available_inode)
    }

    #[deprecated(note = "probably bad to manipulate itree from the outside like this")]
    pub fn promote_next_inode(&self, new: Ino) -> WhResult<()> {
        let next = &mut self.itree.write().next_ino;
        new.ge(&next.start)
            .then_some(())
            .ok_or(WhError::InodeNotFound)
            .inspect_err(|_| log::error!("Ran out of Ino, returning Wh::WouldBlock"))?;
        *next = new..;
        // .next()
        // .ok_or(WhError::WouldBlock { called_from: "get_next_inode".to_owned()} )
        // .inspect_err(|e| log::error!("Ran out of Ino, returning Wh::WouldBlock"))
        Ok(())

        // let mut next_inode =
        //     self.next_inode
        //         .try_lock_for(LOCK_TIMEOUT)
        //         .ok_or(WhError::WouldBlock {
        //             called_from: "promote_next_inode".to_string(),
        //         })?;

        // // REVIEW: next_inode being behind a mutex is weird and
        // // the function not taking a mutable ref feels weird, is next_inode behind a mutex just to allow a simple &self?
        // if *next_inode < new {
        //     *next_inode = new;
        // };
        // Ok(())
    }

    #[must_use]
    /// Add the requested entry to the itree and inform the network
    pub fn register_new_inode(&self, inode: Inode) -> Result<(), MakeInodeError> {
        ITree::write_lock(&self.itree, "register_new_inode")?.add_inode(inode.clone())?;

        if !ITree::is_local_only(inode.id) {
            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(MessageContent::Inode(
                    inode,
                )))
                .expect("register inode: unable to update modification on the network thread");
        }
        Ok(())
        // TODO - if unable to update for some reason, should be passed to the background worker
    }

    pub fn rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: InodeName,
        new_name: InodeName,
        overwrite: bool,
    ) -> Result<(), RenameError> {
        let mut itree = ITree::write_lock(&self.itree, "itree_rename_file")?;

        itree.mv_inode(parent, new_parent, name.as_ref(), new_name.clone())?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(MessageContent::Rename(
                parent, new_parent, name, new_name, overwrite,
            )))
            .expect("broadcast_rename_file: unable to update modification on the network thread");
        Ok(())
    }

    pub fn acknowledge_rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
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
    pub fn acknowledge_new_file(&self, inode: Inode, _id: InodeId) -> Result<(), MakeInodeError> {
        let mut itree = ITree::write_lock(&self.itree, "acknowledge_new_file")?;
        itree.add_inode(inode)
    }

    /// Remove [Inode] from the [ITree] and inform the network of the removal
    pub fn unregister_inode(&self, id: InodeId) -> Result<(), RemoveInodeError> {
        ITree::write_lock(&self.itree, "unregister_inode")?.remove_inode(id)?;

        if !ITree::is_local_only(id) {
            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(MessageContent::Remove(
                    id,
                )))
                .expect("unregister_inode: unable to update modification on the network thread");
        }
        // TODO - if unable to update for some reason, should be passed to the background worker
        Ok(())
    }

    /// Remove [Inode] from the [ITree]
    pub fn acknowledge_unregister_inode(&self, id: InodeId) -> Result<Inode, RemoveInodeError> {
        ITree::write_lock(&self.itree, "acknowledge_unregister_inode")?.remove_inode(id)
    }

    pub fn acknowledge_hosts_edition(&self, id: InodeId, hosts: Vec<Address>) -> WhResult<()> {
        let mut itree = ITree::write_lock(&self.itree, "acknowledge_hosts_edition")?;

        itree.set_inode_hosts(id, hosts) // TODO - if unable to update for some reason, should be passed to the background worker
    }

    pub fn send_file(&self, inode: InodeId, data: Vec<u8>, to: Address) -> WhResult<()> {
        self.to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                (MessageContent::PullAnswer(inode, data), None),
                vec![to],
            ))
            .expect("send_file: unable to update modification on the network thread");
        Ok(())
    }

    pub fn revoke_remote_hosts(&self, id: InodeId) -> WhResult<()> {
        self.update_hosts(id, vec![self.hostname()?])?;
        // self.apply_redundancy(id);
        Ok(())
    }

    pub fn add_inode_hosts(&self, ino: InodeId, hosts: Vec<Address>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "network_interface::update_hosts")?
            .add_inode_hosts(ino, hosts)?;
        self.update_remote_hosts(ino)
    }

    pub fn update_hosts(&self, ino: InodeId, hosts: Vec<Address>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "network_interface::update_hosts")?
            .set_inode_hosts(ino, hosts)?;
        self.update_remote_hosts(ino)
    }

    fn update_remote_hosts(&self, ino: InodeId) -> WhResult<()> {
        let inode = ITree::read_lock(&self.itree, "update_remote_hosts")?
            .get_inode(ino)?
            .clone();

        if let FsEntry::File(hosts) = &inode.entry {
            if !ITree::is_local_only(inode.id) {
                self.to_network_message_tx
                    .send(ToNetworkMessage::BroadcastMessage(
                        MessageContent::EditHosts(inode.id, hosts.clone()),
                    ))
                    .expect(
                        "update_remote_hosts: unable to update modification on the network thread",
                    );
            }
            Ok(())
        } else {
            Err(WhError::InodeIsADirectory)
        }
    }

    pub fn aknowledge_new_hosts(&self, id: InodeId, new_hosts: Vec<Address>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "aknowledge_new_hosts")?.add_inode_hosts(id, new_hosts)
    }

    pub fn aknowledge_hosts_removal(&self, id: InodeId, new_hosts: Vec<Address>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "aknowledge_hosts_removal")?
            .remove_inode_hosts(id, new_hosts)
    }

    pub fn update_metadata(&self, id: InodeId, meta: Metadata) -> WhResult<()> {
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
                .send(ToNetworkMessage::BroadcastMessage(
                    MessageContent::EditMetadata(id, fixed_meta),
                ))
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

    pub fn apply_redundancy(&self, file_id: InodeId) {
        self.to_redundancy_tx
            .send(RedundancyMessage::ApplyTo(file_id))
            .expect("network_interface::apply_redundancy: tx error");
    }

    // !SECTION ^ Redundancy related

    // SECTION Node related

    // pub fn register_to_others(&self) {
    //     self.to_network_message_tx
    //         .send(ToNetworkMessage::BroadcastMessage(
    //             MessageContent::Register(
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

    // pub async fn request_itree(&self, to: Address) -> io::Result<bool> {
    //     let callback = self.callbacks.create(Callback::PullFs)?;

    //     self.to_network_message_tx
    //         .send(ToNetworkMessage::SpecificMessage(
    //             (MessageContent::RequestFs, None),
    //             vec![to],
    //         ))
    //         .expect("request_itree: unable to update modification on the network thread");

    //     self.callbacks.async_wait_for(callback).await
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

    pub fn send_itree(&self, to: Address, global_config_bytes: Vec<u8>) -> WhResult<()> {
        let itree = ITree::read_lock(&self.itree, "send_itree")?;
        let mut entries = itree.get_raw_entries();

        //Remove ignored entries
        entries.retain(|ino, _| !ITree::is_local_only(*ino));
        entries.entry(1u64).and_modify(|inode| {
            if let FsEntry::Directory(childrens) = &mut inode.entry {
                childrens.retain(|x| !ITree::is_local_only(*x));
            }
        });

        if let Some(peers) = self.peers.try_read_for(LOCK_TIMEOUT) {
            let peers_address_list = peers
                .iter()
                .filter_map(|peer| {
                    if peer.hostname != to {
                        Some(peer.hostname.clone())
                    } else {
                        None
                    }
                })
                .collect();

            self.to_network_message_tx
                .send(ToNetworkMessage::SpecificMessage(
                    (
                        MessageContent::FsAnswer(
                            FileSystemSerialized {
                                fs_index: entries,
                                next_inode: self.get_next_inode()?,
                            },
                            peers_address_list,
                            global_config_bytes,
                        ),
                        None,
                    ),
                    vec![to],
                ))
                .expect("send_itree: unable to update modification on the network thread");
            Ok(())
        } else {
            Err(WhError::WouldBlock { called_from: "send_tree".to_owned() })
        }
    }

    pub fn disconnect_peer(&self, addr: Address) -> WhResult<()> {
        self.peers
            .try_write_for(LOCK_TIMEOUT)
            .ok_or(WhError::WouldBlock { called_from: "disconnect_peer: can't write lock peers".to_owned() })?
            .retain(|p| p.hostname != addr);

        log::debug!("Disconnecting {addr}. Removing from inodes hosts");
        for inode in ITree::write_lock(&self.itree, "disconnect_peer")?.inodes_mut() {
            if let FsEntry::File(hosts) = &mut inode.entry {
                hosts.retain(|h| *h != addr);
            }
        }
        self.to_redundancy_tx
            .send(RedundancyMessage::CheckIntegrity)
            .unwrap();
        Ok(())
    }

    pub async fn network_airport(
        mut receiver: UnboundedReceiver<FromNetworkMessage>,
        fs_interface: Arc<FsInterface>,
    ) {
        loop {
            let FromNetworkMessage { origin, content } = match receiver.recv().await {
                Some(message) => message,
                None => continue,
            };
            if log::log_enabled!(log::Level::Debug) {
                log::debug!("From {}: {:?}", origin, content);
            } else {
                log::info!("From {}: {}", origin, content);
            }
            let content_debug = format!("{content:?}");

            let action_result = match content {
                MessageContent::PullAnswer(id, binary) => fs_interface.recept_binary(id, binary)
                                                            .map_err(into_boxed_io),
                MessageContent::RedundancyFile(id, binary) => fs_interface.recept_redundancy(id, binary)
                                            .map_err(into_boxed_io),
                MessageContent::Inode(inode) => fs_interface.recept_inode(inode).map_err(into_boxed_io),
                MessageContent::EditHosts(id, hosts) => fs_interface.recept_edit_hosts(id, hosts).map_err(into_boxed_io),
                MessageContent::RevokeFile(id, host, meta) => fs_interface.recept_revoke_hosts(id, host, meta).map_err(into_boxed_io),
                MessageContent::AddHosts(id, hosts) => fs_interface.recept_add_hosts(id, hosts).map_err(into_boxed_io),
                MessageContent::RemoveHosts(id, hosts) => {
                                            fs_interface.recept_remove_hosts(id, hosts).map_err(into_boxed_io)
                                        }
                MessageContent::EditMetadata(id, meta) =>
                                            fs_interface.acknowledge_metadata(id, meta).map_err(into_boxed_io),
                MessageContent::Remove(id) => fs_interface.recept_remove_inode(id).map_err(into_boxed_io),
                MessageContent::RequestFile(inode) => fs_interface.send_file(inode, origin).map_err(into_boxed_io),
                MessageContent::RequestFs => fs_interface.send_filesystem(origin).map_err(into_boxed_io),
                MessageContent::Rename(parent, new_parent, name, new_name, overwrite) =>
                                            fs_interface
                                            .recept_rename(parent, new_parent, name, new_name, overwrite)
                                            .map_err(into_boxed_io),
                MessageContent::SetXAttr(ino, key, data) => fs_interface
                                            .network_interface
                                            .recept_inode_xattr(ino, &key, data)
                                            .map_err(into_boxed_io),

                MessageContent::RemoveXAttr(ino, key) => fs_interface
                                            .network_interface
                                            .recept_remove_inode_xattr(ino, &key)
                                            .map_err(into_boxed_io),
                MessageContent::FsAnswer(_, _, _) => {
                                            Err(io::Error::new(ErrorKind::InvalidInput,
                                                "Late answer from first connection, loaded network interface shouldn't recieve FsAnswer"))
                                        },
                MessageContent::Disconnect => fs_interface.network_interface.disconnect_peer(origin).map_err(into_boxed_io),
                MessageContent::FileDelta(ino, meta, sig, delta) => fs_interface.accept_delta(ino, meta, sig, delta, origin)
                                            .map_err(into_boxed_io),
                MessageContent::FileChanged(ino, meta) => fs_interface.accept_file_changed(ino, meta, origin).map_err(into_boxed_io),
                MessageContent::DeltaRequest(ino, sig) => fs_interface.respond_delta(ino, sig, origin).map_err(into_boxed_io),
            };
            if let Err(error) = action_result {
                log::error!(
                    "Network airport couldn't operate operation {content_debug}, error found: {error}"
                );
            }
        }
    }

    pub async fn contact_peers(
        peers_list: Arc<RwLock<Vec<PeerIPC>>>,
        mut rx: UnboundedReceiver<ToNetworkMessage>,
    ) {
        log::info!("contact peers");
        while let Some(message) = rx.recv().await {
            // geeting all peers network senders
            let peers_tx: Vec<(UnboundedSender<MessageAndStatus>, String)> = peers_list
                .try_read_for(LOCK_TIMEOUT)
                .expect("mutext error on contact_peers") // TODO - handle timeout
                .iter()
                .map(|peer| (peer.sender.clone(), peer.hostname.clone()))
                .collect();

            match message {
                ToNetworkMessage::BroadcastMessage(message_content) => {
                    peers_tx.iter().for_each(|(channel, address)| {
                        channel
                            .send((message_content.clone(), None))
                            .expect(&format!("failed to send message to peer {}", address))
                    });
                }
                ToNetworkMessage::SpecificMessage((message_content, status_tx), origins) => {
                    let count = peers_tx
                        .iter()
                        .filter(|&(_, address)| origins.contains(address))
                        .map(|(channel, address)| {
                            channel
                                .send((message_content.clone(), status_tx.clone())) // warning: only the first peer channel can set a status
                                .expect(&format!("failed to send message to peer {}", address))
                        })
                        .count();
                    if count == 0 {
                        log::warn!(
                            "contact_peers: {message_content}: No peers by hostname {origins:?}"
                        )
                    }
                }
            };
        }
    }

    pub async fn incoming_connections_watchdog(
        server: Arc<Server>,
        receiver_in: UnboundedSender<FromNetworkMessage>,
        network_interface: Arc<NetworkInterface>,
    ) {
        while let Ok((stream, addr)) = server.listener.accept().await {
            log::debug!("GOT ADDRESS {addr}");
            let ws_stream = tokio_tungstenite::accept_async_with_config(
                stream,
                Some(
                    WebSocketConfig::default()
                        .max_message_size(None)
                        .max_frame_size(None),
                ),
            )
            .await
            .expect("Error during the websocket handshake occurred");

            match PeerIPC::accept(&network_interface, ws_stream, receiver_in.clone()).await {
                Ok(new_peer) => network_interface
                    .peers
                    .try_write_for(LOCK_TIMEOUT)
                    .expect("incoming_connections_watchdog: can't lock existing peers")
                    .push(new_peer),
                Err(e) => log::error!("incomming: accept: {e}"),
            }
        }
    }

    // !SECTION ^ Node related
}
