use std::{collections::HashMap, io, sync::Arc};

use parking_lot::{Mutex, RwLock};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::{arbo::FsEntry, whpath::WhPath};
use crate::network::{
    message::{self, Address, FromNetworkMessage, MessageContent, ToNetworkMessage},
    peer_ipc::PeerIPC,
    server::Server,
};

use super::{
    arbo::{Arbo, Inode, InodeId, LOCK_TIMEOUT},
    fs_interface::FsInterface,
};

#[derive(Eq, Hash, PartialEq)]
pub enum Callback {
    Pull(InodeId),
}

pub struct NetworkInterface {
    pub arbo: Arc<RwLock<Arbo>>,
    pub mount_point: WhPath,
    pub to_network_message_tx: UnboundedSender<ToNetworkMessage>,
    pub next_inode: Mutex<InodeId>, // TODO - replace with InodeIndex type
    pub callbacks: RwLock<HashMap<Callback, UnboundedSender<bool>>>,
    pub peers: Arc<RwLock<Vec<PeerIPC>>>,
    self_addr: Address,
}

impl NetworkInterface {
    pub fn new(
        arbo: Arc<RwLock<Arbo>>,
        mount_point: WhPath,
        to_network_message_tx: UnboundedSender<ToNetworkMessage>,
        next_inode: InodeId,
        self_addr: Address,
    ) -> Self {
        let next_inode = Mutex::new(next_inode);

        Self {
            arbo,
            mount_point,
            to_network_message_tx,
            next_inode,
            callbacks: RwLock::new(HashMap::new()),
            peers: Arc::new(RwLock::new(vec![])),
            self_addr,
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

    #[must_use]
    /// add the requested entry to the arbo and inform the network
    pub fn register_new_file(&self, inode: Inode) -> io::Result<u64> {
        let new_inode_id = inode.id;

        if let Some(mut arbo) = self.arbo.try_write_for(LOCK_TIMEOUT) {
            arbo.add_inode(inode.clone())?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "mkfile: can't write-lock arbo's RwLock",
            ));
        };

        // TODO - add myself to hosts

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                message::MessageContent::Inode(inode, new_inode_id),
            ))
            .expect("mkfile: unable to update modification on the network thread");
        // TODO - if unable to update for some reason, should be passed to the background worker

        Ok(new_inode_id)
    }

    #[must_use]
    /// Get a new inode, add the requested entry to the arbo and inform the network
    pub fn acknowledge_new_file(&self, inode: Inode, id: InodeId) -> io::Result<()> {
        if let Some(mut arbo) = self.arbo.try_write_for(LOCK_TIMEOUT) {
            match arbo.add_inode(inode) {
                Ok(()) => (),
                Err(_) => todo!("acknowledge_new_file: file already existing: conflict"), // TODO
            };
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "mkfile: can't write-lock arbo's RwLock",
            ));
        };

        Ok(())
    }

    /// remove the requested entry to the arbo and inform the network
    pub fn unregister_file(&self, id: InodeId) -> io::Result<Inode> {
        let removed_inode: Inode;

        if let Some(mut arbo) = self.arbo.try_write_for(LOCK_TIMEOUT) {
            removed_inode = arbo.remove_inode(id)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "mkfile: can't write-lock arbo's RwLock",
            ));
        };

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                message::MessageContent::Remove(id),
            ))
            .expect("mkfile: unable to update modification on the network thread");

        // TODO - if unable to update for some reason, should be passed to the background worker

        Ok(removed_inode)
    }

    pub fn acknowledge_unregister_file(&self, id: InodeId) -> io::Result<Inode> {
        let removed_inode: Inode;

        if let Some(mut arbo) = self.arbo.try_write_for(LOCK_TIMEOUT) {
            removed_inode = arbo.remove_inode(id)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "mkfile: can't write-lock arbo's RwLock",
            ));
        };

        Ok(removed_inode)
    }

    pub fn acknowledge_hosts_edition(&self, id: InodeId, hosts: Vec<Address>) -> io::Result<()> {
        let mut arbo = Arbo::write_lock(&self.arbo, "acknowledge_hosts_edition")?;

        arbo.set_inode_hosts(id, hosts) // TODO - if unable to update for some reason, should be passed to the background worker
    }

    // REVIEW - recheck and simplify this if possible
    pub fn pull_file(&self, file: InodeId) -> io::Result<UnboundedReceiver<bool>> {
        if let Some(arbo) = self.arbo.try_read_for(LOCK_TIMEOUT) {
            if let FsEntry::File(hosts) = &arbo.get_inode(file)?.entry {
                let (callback_tx, callback_rx) = mpsc::unbounded_channel::<bool>();
                if hosts.contains(&self.self_addr) {
                    callback_tx
                        .send(true) // directly completes the callback
                        .expect("pull_file: unable to callback");
                    return Ok(callback_rx);
                }
                self.to_network_message_tx
                    .send(ToNetworkMessage::SpecificMessage(
                        message::MessageContent::RequestFile(file),
                        vec![hosts[0].clone()], // NOTE - dumb choice for now
                    ))
                    .expect("pull_file: unable to request on the network thread");
                if let Some(mut waiting_dl) = self.callbacks.try_write_for(LOCK_TIMEOUT) {
                    if let Some(old_callback_tx) =
                        waiting_dl.insert(Callback::Pull(file), callback_tx)
                    {
                        old_callback_tx.send(false); // TODO - actually the old callback is droped on fail. not optimal
                    }
                    Ok(callback_rx)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "pull_file: can't write lock self.callbacks",
                    ))
                }
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "pull_file: can't pull a folder",
                ))
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "pull_file: can't read lock arbo's RwLock",
            ))
        }
    }

    pub fn resolve_pull(&self, id: InodeId, status: bool) {
        if let Some(callback_tx) = self
            .callbacks
            .try_write_for(LOCK_TIMEOUT)
            .expect("can't lock callbacks and resolve pull") // TODO - manage
            .remove(&Callback::Pull(id))
        {
            callback_tx.send(status);
        }
    }

    pub fn revoke_remote_hosts(&self, id: InodeId) -> io::Result<()> {
        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::EditHosts(id, vec![self.self_addr.clone()]),
            ))
            .expect("revoke_remote_hosts: unable to update modification on the network thread");
        Ok(())
        /* REVIEW
         * This system (and others broadcasts systems) should be reviewed as they don't check success.
         * In this case, if another host misses this order, it will not update it's file.
         * We could create a "broadcast" callback with the number of awaited confirmations and a timeout
         * before resend or fail declaration.
         * Or send a bunch of Specific messages
         */
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

            match content {
                MessageContent::PullAnswer(id, binary) => {
                    fs_interface.recept_binary(id, binary);
                }
                MessageContent::Binary(bin) => {
                    println!("peer: {:?}", String::from_utf8(bin).unwrap_or_default());
                }
                MessageContent::Inode(inode, id) => {
                    fs_interface.recept_inode(inode, id);
                }
                MessageContent::EditHosts(id, hosts) => {
                    fs_interface.recept_edit_hosts(id, hosts);
                }
                MessageContent::Remove(ino) => {
                    todo!();
                    //let mut provider = provider.lock().expect("failed to lock mutex");
                    //provider.recpt_remove(ino);
                }
                MessageContent::Write(ino, data) => {
                    todo!();
                    // deprecated ?
                    //let mut provider = provider.lock().expect("failed to lock mutex");
                    //provider.recpt_write(ino, data);
                }
                MessageContent::Meta(_) => {}
                MessageContent::RequestFile(_) => {}
                MessageContent::RequestFs => {
                    todo!();
                    //let provider = provider.lock().expect("failed to lock mutex");
                    //provider.send_file_system(origin);
                }
                MessageContent::FileStructure(fs) => {
                    todo!();
                    //let mut provider = provider.lock().expect("failed to lock mutex");
                    //provider.merge_file_system(fs);
                }
            };
        }
    }

    pub async fn contact_peers(
        peers_list: Arc<RwLock<Vec<PeerIPC>>>,
        mut rx: UnboundedReceiver<ToNetworkMessage>,
    ) {
        // on message reception, broadcast it to all peers senders
        while let Some(message) = rx.recv().await {
            let peer_tx: Vec<(UnboundedSender<MessageContent>, String)> = peers_list
                .try_read_for(LOCK_TIMEOUT)
                .unwrap() // TODO - handle timeout
                .iter()
                .map(|peer| (peer.sender.clone(), peer.address.clone()))
                .collect();

            println!("broadcasting message to peers:\n{:?}", message);
            match message {
                ToNetworkMessage::BroadcastMessage(message_content) => {
                    peer_tx.iter().for_each(|(channel, address)| {
                        println!("peer: {}", address);
                        channel
                            .send(message_content.clone())
                            .expect(&format!("failed to send message to peer {}", address))
                    });
                }
                ToNetworkMessage::SpecificMessage(message_content, origins) => {
                    peer_tx
                        .iter()
                        .filter(|&(_, address)| origins.contains(address))
                        .for_each(|(channel, address)| {
                            println!("peer: {}", address);
                            channel
                                .send(message_content.clone())
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
                    .unwrap() // TODO - handle timeout
                    .push(new_peer);
            }
        }
    }
}
