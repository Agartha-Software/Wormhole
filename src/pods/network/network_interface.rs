use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use parking_lot::{Mutex, RwLock};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    error::WhError,
    pods::{
        arbo::{FsEntry, Metadata, BLOCK_SIZE},
        filesystem::{
            make_inode::MakeInodeError, remove_inode::RemoveInodeError, rename::RenameError,
        },
        network::callbacks::Callback,
        whpath::WhPath,
    },
};
use crate::{
    error::WhResult,
    network::{
        message::{
            self, Address, FileSystemSerialized, FromNetworkMessage, MessageAndStatus,
            MessageContent, ToNetworkMessage,
        },
        peer_ipc::PeerIPC,
        server::Server,
    },
};

use crate::pods::{
    arbo::{Arbo, Inode, InodeId, LOCK_TIMEOUT},
    filesystem::fs_interface::FsInterface,
};

use crate::pods::network::callbacks::Callbacks;

pub struct NetworkInterface {
    pub arbo: Arc<RwLock<Arbo>>,
    pub mount_point: WhPath,
    pub to_network_message_tx: UnboundedSender<ToNetworkMessage>,
    pub next_inode: Mutex<InodeId>, // TODO - replace with InodeIndex type
    pub callbacks: Callbacks,
    pub peers: Arc<RwLock<Vec<PeerIPC>>>,
    pub self_addr: Address,
    pub redundancy: u64,
}

impl NetworkInterface {
    pub fn new(
        arbo: Arc<RwLock<Arbo>>,
        mount_point: WhPath,
        to_network_message_tx: UnboundedSender<ToNetworkMessage>,
        next_inode: InodeId,
        peers: Arc<RwLock<Vec<PeerIPC>>>,
        self_addr: Address,
        redundancy: u64,
    ) -> Self {
        let next_inode = Mutex::new(next_inode);

        Self {
            arbo,
            mount_point,
            to_network_message_tx,
            next_inode,
            callbacks: Callbacks {
                callbacks: HashMap::new().into(),
            },
            peers,
            self_addr,
            redundancy,
        }
    }

    pub fn get_next_inode(&self) -> io::Result<u64> {
        let mut next_inode = match self.next_inode.try_lock_for(LOCK_TIMEOUT) {
            Some(lock) => Ok(lock),
            None => Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "get_next_inode: can't lock next_inode",
            )),
        }?;
        let available_inode = *next_inode;
        *next_inode += 1;

        Ok(available_inode)
    }

    /** TODO: Doc when reviews are finished */
    pub fn n_get_next_inode(&self) -> WhResult<u64> {
        let mut next_inode =
            self.next_inode
                .try_lock_for(LOCK_TIMEOUT)
                .ok_or(WhError::WouldBlock {
                    called_from: "get_next_inode".to_string(),
                })?;

        let available_inode = *next_inode;
        *next_inode += 1;

        Ok(available_inode)
    }

    #[must_use]
    pub fn promote_next_inode(&self, new: u64) -> WhResult<()> {
        let mut next_inode =
            self.next_inode
                .try_lock_for(LOCK_TIMEOUT)
                .ok_or(WhError::WouldBlock {
                    called_from: "promote_next_inode".to_string(),
                })?;

        // REVIEW: next_inode being behind a mutex is weird and
        // the function not taking a mutable ref feels weird, is next_inode behind a mutex just to allow a simple &self?
        if *next_inode < new {
            *next_inode = new;
        };
        Ok(())
    }

    #[must_use]
    /// Add the requested entry to the arbo and inform the network
    pub fn register_new_inode(&self, inode: Inode) -> Result<(), MakeInodeError> {
        let inode_id = inode.id.clone();
        Arbo::n_write_lock(&self.arbo, "register_new_inode")?.n_add_inode(inode.clone())?;

        if !Arbo::is_local_only(inode_id) {
            if matches!(inode.entry, FsEntry::File(_)) {
                self.apply_redundancy(inode_id)?;
            }

            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(
                    message::MessageContent::Inode(inode),
                ))
                .expect("register inode: unable to update modification on the network thread");
        }
        Ok(())
        // TODO - if unable to update for some reason, should be passed to the background worker
    }

    pub fn n_rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: &String,
        new_name: &String,
        overwrite: bool,
    ) -> Result<(), RenameError> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "arbo_rename_file")?;

        arbo.n_mv_inode(parent, new_parent, name, new_name)?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                message::MessageContent::Rename(
                    parent,
                    new_parent,
                    name.clone(),
                    new_name.clone(),
                    overwrite,
                ),
            ))
            .expect("broadcast_rename_file: unable to update modification on the network thread");
        Ok(())
    }

    pub fn acknowledge_rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: &String,
        new_name: &String,
    ) -> Result<(), RenameError> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "arbo_rename_file")?;

        arbo.n_mv_inode(parent, new_parent, name, new_name)
            .map_err(|err| match err {
                WhError::InodeNotFound => RenameError::DestinationParentNotFound,
                WhError::InodeIsNotADirectory => RenameError::DestinationParentNotFolder,
                source => RenameError::WhError { source },
            })
    }

    #[must_use]
    /// Get a new inode, add the requested entry to the arbo and inform the network
    pub fn acknowledge_new_file(&self, inode: Inode, _id: InodeId) -> Result<(), MakeInodeError> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "acknowledge_new_file")?;
        arbo.n_add_inode(inode)
    }

    /// Remove [Inode] from the [Arbo] and inform the network of the removal
    pub fn unregister_inode(&self, id: InodeId) -> Result<(), RemoveInodeError> {
        Arbo::n_write_lock(&self.arbo, "unregister_inode")?.n_remove_inode(id)?;

        if !Arbo::is_local_only(id) {
            self.to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(
                    message::MessageContent::Remove(id),
                ))
                .expect("unregister_inode: unable to update modification on the network thread");
        }
        // TODO - if unable to update for some reason, should be passed to the background worker
        Ok(())
    }

    /// Remove [Inode] from the [Arbo]
    pub fn acknowledge_unregister_inode(&self, id: InodeId) -> Result<Inode, RemoveInodeError> {
        Arbo::n_write_lock(&self.arbo, "acknowledge_unregister_inode")?.n_remove_inode(id)
    }

    pub fn acknowledge_hosts_edition(&self, id: InodeId, hosts: Vec<Address>) -> WhResult<()> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "acknowledge_hosts_edition")?;

        arbo.n_set_inode_hosts(id, hosts) // TODO - if unable to update for some reason, should be passed to the background worker
    }

    pub fn acknowledge_metadata(&self, id: InodeId, meta: Metadata) -> WhResult<()> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "acknowledge_metadata")?;

        arbo.n_set_inode_meta(id, meta) // TODO - if unable to update for some reason, should be passed to the background worker
    }

    pub fn send_file(&self, inode: InodeId, data: Vec<u8>, to: Address) -> io::Result<()> {
        self.to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                (MessageContent::PullAnswer(inode, data), None),
                vec![to],
            ))
            .expect("send_file: unable to update modification on the network thread");
        Ok(())
    }

    fn affect_write_locally(&self, id: InodeId, new_size: usize) -> WhResult<Metadata> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "network_interface.affect_write_locally")?;
        let inode = arbo.n_get_inode_mut(id)?;


        let new_size = new_size.max(inode.meta.size as usize);
        inode.meta.size = new_size as u64;
        inode.meta.blocks = ((new_size + BLOCK_SIZE - 1) / BLOCK_SIZE) as u64;

        inode.meta.mtime = SystemTime::now();

        inode.entry = match &inode.entry {
            FsEntry::File(_) => FsEntry::File(vec![self.self_addr.clone()]),
            _ => panic!("Can't edit hosts on folder"),
        };
        Ok(inode.meta.clone())
    }

    pub fn write_file(
        &self,
        id: InodeId,
        new_size: usize,
    ) -> WhResult<()> {
        let meta = self.affect_write_locally(id, new_size)?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::RevokeFile(id, self.self_addr.clone(), meta),
            ))
            .expect("revoke_remote_hosts: unable to update modification on the network thread");
        self.apply_redundancy(id)
    }

    pub fn update_remote_hosts(&self, inode: &Inode) -> io::Result<()> {
        if let FsEntry::File(hosts) = &inode.entry {
            if !Arbo::is_local_only(inode.id) {
                self.to_network_message_tx
                    .send(ToNetworkMessage::BroadcastMessage(
                        MessageContent::EditHosts(inode.id, hosts.clone()),
                    ))
                    .expect("update_remote_hosts: unable to update modification on the network thread");
            }
            Ok(())
        } else {
            Err(io::ErrorKind::InvalidInput.into())
        }
    }

    pub fn aknowledge_new_hosts(&self, id: InodeId, new_hosts: Vec<Address>) -> io::Result<()> {
        Arbo::write_lock(&self.arbo, "aknowledge_new_hosts")?.add_inode_hosts(id, new_hosts)
    }

    pub fn aknowledge_hosts_removal(&self, id: InodeId, new_hosts: Vec<Address>) -> io::Result<()> {
        Arbo::write_lock(&self.arbo, "aknowledge_hosts_removal")?.remove_inode_hosts(id, new_hosts)
    }

    pub fn update_metadata(&self, id: InodeId, meta: Metadata) -> io::Result<()> {
        let mut arbo = Arbo::write_lock(&self.arbo, "network_interface::update_metadata")?;
        let mut fixed_meta = meta;
        let ref_meta = arbo.get_inode(id)?.meta.clone();

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

        arbo.set_inode_meta(id, fixed_meta.clone())?;
        drop(arbo);

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::EditMetadata(id, fixed_meta),
            ))
            .expect("update_metadata: unable to update modification on the network thread");
        Ok(())
        /* REVIEW
         * This system (and others broadcasts systems) should be reviewed as they don't check success.
         * In this case, if another host misses this order, it will not update it's file.
         * We could create a "broadcast" callback with the number of awaited confirmations and a timeout
         * before resend or fail declaration.
         * Or send a bunch of Specific messages
         */
    }

    pub fn n_update_metadata(&self, id: InodeId, meta: Metadata) -> WhResult<()> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "network_interface::update_metadata")?;
        let mut fixed_meta = meta;
        let ref_meta = arbo.n_get_inode(id)?.meta.clone();

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

        arbo.n_set_inode_meta(id, fixed_meta.clone())?;
        drop(arbo);

        if !Arbo::is_local_only(id) {
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

    fn add_redundancy(&self, file_id: InodeId, current_hosts: Vec<Address>) -> WhResult<()> {
        let possible_hosts: Vec<Address> = self
            .peers
            .try_read_for(LOCK_TIMEOUT)
            .ok_or(WhError::WouldBlock {
                called_from: "apply_redundancy: can't lock peers mutex".to_string(),
            })?
            .iter()
            .map(|peer| peer.address.clone())
            .filter(|addr| !current_hosts.contains(addr))
            .take(self.redundancy as usize - current_hosts.len())
            .collect::<Vec<Address>>();

        if (current_hosts.len() + possible_hosts.len()) < self.redundancy as usize {
            log::warn!("redundancy needs enough hosts");
            return Ok(()); // TODO - should be handled (is not ok)
        }

        self.to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                (MessageContent::RequestPull(file_id), None),
                possible_hosts,
            ))
            .expect("apply_redundancy: unable to send redundancy on the network thread");
        Ok(())
    }

    fn remove_redundancy(&self, file_id: InodeId, current_hosts: Vec<Address>) {
        let hosts_nb = current_hosts.len();
        let discard_hosts: Vec<String> = current_hosts
            .into_iter()
            .filter(|addr| *addr != self.self_addr)
            .take(hosts_nb - self.redundancy as usize)
            .collect();

        // NOTE - removing hosts also remove their file on disk upon reception
        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::RemoveHosts(file_id, discard_hosts),
            ))
            .expect("apply_redundancy: unable to send discard redundancy on the network thread");
    }

    pub fn apply_redundancy(&self, file_id: InodeId) -> WhResult<()> {
        let current_hosts: Vec<String> = {
            let arbo = Arbo::n_read_lock(&self.arbo, "apply_redundancy")?;

            if let FsEntry::File(current_hosts) = &arbo.n_get_inode(file_id)?.entry {
                current_hosts.clone()
            } else {
                panic!("Can't apply redundancy to a folder");
            }
        };

        if current_hosts.len() < self.redundancy as usize {
            self.add_redundancy(file_id, current_hosts)?
        } else if current_hosts.len() > self.redundancy as usize {
            self.remove_redundancy(file_id, current_hosts)
        }
        Ok(())
    }

    // !SECTION ^ Redundancy related

    // SECTION Node related

    pub fn register_to_others(&self) {
        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::Register(self.self_addr.clone()),
            ))
            .expect("register_to_others: unable to update modification on the network thread");
    }

    pub async fn request_arbo(&self, to: Address) -> io::Result<bool> {
        let callback = self.callbacks.create(Callback::PullFs)?;

        self.to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                (MessageContent::RequestFs, None),
                vec![to],
            ))
            .expect("request_arbo: unable to update modification on the network thread");

        self.callbacks.async_wait_for(callback).await
    }

    pub fn edit_peer_ip(&self, actual: Address, new: Address) {
        log::info!("changing host {} to {}", actual, new);
        if let Some(mut peers) = self.peers.try_write_for(LOCK_TIMEOUT) {
            for peer in peers.iter_mut() {
                if peer.address == actual {
                    log::info!("done once");
                    peer.address = new.clone();
                }
            }
        }
    }

    pub fn send_arbo(&self, to: Address) -> io::Result<()> {
        let arbo = Arbo::read_lock(&self.arbo, "send_arbo")?;
        let mut entries = arbo.get_raw_entries();

        //Remove ignored entries
        entries.retain(|ino, _| !Arbo::is_local_only(*ino));
        entries.entry(1u64).and_modify(|inode| {
            if let FsEntry::Directory(childrens) = &mut inode.entry {
                childrens.retain(|x| !Arbo::is_local_only(*x));
            }
        });

        if let Some(peers) = self.peers.try_read_for(LOCK_TIMEOUT) {
            let peers_address_list = peers.iter().map(|peer| peer.address.clone()).collect();

            self.to_network_message_tx
                .send(ToNetworkMessage::SpecificMessage(
                    (
                        MessageContent::FsAnswer(
                            FileSystemSerialized {
                                fs_index: entries,
                                next_inode: self.get_next_inode()?,
                            },
                            peers_address_list,
                        ),
                        None,
                    ),
                    vec![to],
                ))
                .expect("send_arbo: unable to update modification on the network thread");
            Ok(())
        } else {
            Err(std::io::Error::new(
                io::ErrorKind::WouldBlock,
                "Deadlock while trying to read peers",
            ))
        }
    }

    pub fn register_new_node(&self, socket: Address, addr: Address) {
        self.edit_peer_ip(socket, addr);
    }

    pub async fn network_airport(
        mut network_reception: UnboundedReceiver<FromNetworkMessage>,
        fs_interface: Arc<FsInterface>,
    ) {
        loop {
            let FromNetworkMessage { origin, content } = match network_reception.recv().await {
                Some(message) => message,
                None => continue,
            };
            log::debug!("From {}: {:?}", origin, content);
            let content_name = content.to_string();

            let action_result = match content { // remove scary clone
                MessageContent::PullAnswer(id, binary) => fs_interface.recept_binary(id, binary),
                MessageContent::Inode(inode) => fs_interface.recept_inode(inode).or_else(|err| {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        ))
                    }),
                MessageContent::EditHosts(id, hosts) => fs_interface.recept_edit_hosts(id, hosts).or_else(|err| {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        ))
                    }),
                MessageContent::RevokeFile(id, host, meta) => fs_interface.recept_revoke_hosts(id, host, meta).or_else(|err| {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        ))
                    }),
                MessageContent::AddHosts(id, hosts) => fs_interface.recept_add_hosts(id, hosts),
                MessageContent::RemoveHosts(id, hosts) => {
                    fs_interface.recept_remove_hosts(id, hosts)
                }
                MessageContent::EditMetadata(id, meta) =>
                    fs_interface.network_interface.acknowledge_metadata(id, meta).or_else(|err| {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        ))
                    }),
                MessageContent::Remove(id) => fs_interface.recept_remove_inode(id).or_else(|err| {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        ))
                    }),
                MessageContent::RequestFile(inode, peer) => fs_interface.send_file(inode, peer),
                MessageContent::RequestFs => fs_interface.send_filesystem(origin),
                MessageContent::Register(addr) => Ok(fs_interface.register_new_node(origin, addr)),
                MessageContent::Rename(parent, new_parent, name, new_name, overwrite) =>
                    fs_interface
                    .recept_rename(parent, new_parent, &name, &new_name, overwrite)
                    .map_err(|err| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        )
                    }),
                MessageContent::RequestPull(id) => fs_interface.pull_file_async(id).await,
                MessageContent::SetXAttr(ino, key, data) => fs_interface
                    .network_interface
                    .recept_inode_xattr(ino, key, data)
                    .or_else(|err| {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        ))
                    }),
                MessageContent::RemoveXAttr(ino, key) => fs_interface
                    .network_interface
                    .recept_remove_inode_xattr(ino, key)
                    .or_else(|err| {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("WhError: {err}"),
                        ))
                    }),
                MessageContent::FsAnswer(_, _) => {
                    Err(io::Error::new(ErrorKind::InvalidInput,
                        "Late answer from first connection, loaded network interface shouldn't recieve FsAnswer"))
                }
            };
            if let Err(error) = action_result {
                log::error!(
                    "Network airport couldn't operate operation {content_name:?}, error found: {error}"
                );
            }
        }
    }

    pub async fn contact_peers(
        peers_list: Arc<RwLock<Vec<PeerIPC>>>,
        mut rx: UnboundedReceiver<ToNetworkMessage>,
    ) {
        while let Some(message) = rx.recv().await {
            // geeting all peers network senders
            let peers_tx: Vec<(UnboundedSender<MessageAndStatus>, String)> = peers_list
                .try_read_for(LOCK_TIMEOUT)
                .expect("mutext error on contact_peers") // TODO - handle timeout
                .iter()
                .map(|peer| (peer.sender.clone(), peer.address.clone()))
                .collect();

            log::info!("broadcasting message to peers:\n{:?}", message);
            log::info!(
                "peers list {:#?}",
                peers_list
                    .read()
                    .iter()
                    .map(|peer| peer.address.clone())
                    .collect::<Vec<String>>()
            );
            match message {
                ToNetworkMessage::BroadcastMessage(message_content) => {
                    peers_tx.iter().for_each(|(channel, address)| {
                        channel
                            .send((message_content.clone(), None))
                            .expect(&format!("failed to send message to peer {}", address))
                    });
                }
                ToNetworkMessage::SpecificMessage((message_content, status_tx), origins) => {
                    peers_tx
                        .iter()
                        .filter(|&(_, address)| origins.contains(address))
                        .for_each(|(channel, address)| {
                            channel
                                .send((message_content.clone(), status_tx.clone()))
                                .expect(&format!("failed to send message to peer {}", address))
                        });
                }
            };
        }
    }

    pub async fn incoming_connections_watchdog(
        server: Arc<Server>,
        nfa_tx: UnboundedSender<FromNetworkMessage>,
        existing_peers: Arc<RwLock<Vec<PeerIPC>>>,
    ) {
        while let Ok((stream, _)) = server.listener.accept().await {
            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("Error during the websocket handshake occurred");
            let addr = ws_stream.get_ref().peer_addr().unwrap().to_string();

            let (write, read) = futures_util::StreamExt::split(ws_stream);
            let new_peer = PeerIPC::connect_from_incomming(addr, nfa_tx.clone(), write, read);
            {
                existing_peers
                    .try_write_for(LOCK_TIMEOUT)
                    .expect("incoming_connections_watchdog: can't lock existing peers")
                    .push(new_peer);
            }
        }
    }

    // !SECTION ^ Node related
}
