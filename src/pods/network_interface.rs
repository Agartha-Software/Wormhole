use std::{io, sync::Arc};

use parking_lot::{Mutex, RwLock};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::{
    network::{
        message::{self, FromNetworkMessage, MessageContent, ToNetworkMessage},
        peer_ipc::PeerIPC,
        server::Server,
    },
    providers::whpath::WhPath,
};

use super::{
    arbo::{Arbo, Inode, InodeId, LOCK_TIMEOUT},
    fs_interface::FsInterface,
};

pub struct NetworkInterface {
    pub arbo: Arc<RwLock<Arbo>>,
    pub mount_point: WhPath, // TODO - replace by Ludo's unipath
    pub to_network_message_tx: UnboundedSender<ToNetworkMessage>,
    pub next_inode: Mutex<InodeId>, // TODO - replace with InodeIndex type
    network_airport_handle: Option<JoinHandle<()>>,
    peer_broadcast_handle: Option<JoinHandle<()>>,
    new_peer_handle: Option<JoinHandle<()>>,
    peers: Arc<RwLock<Vec<PeerIPC>>>,
}

impl NetworkInterface {
    pub fn new(
        arbo: Arc<RwLock<Arbo>>,
        mount_point: WhPath,
        to_network_message_tx: UnboundedSender<ToNetworkMessage>,
        next_inode: InodeId,
    ) -> Self {
        let next_inode = Mutex::new(next_inode);

        Self {
            arbo,
            mount_point,
            to_network_message_tx,
            next_inode,
            network_airport_handle: None,
            peer_broadcast_handle: None,
            new_peer_handle: None,
            peers: Arc::new(RwLock::new(vec![])),
        }
    }

    pub fn start_network_airport(
        &self,
        fs_interface: Arc<FsInterface>,
        from_network_message_rx: UnboundedReceiver<FromNetworkMessage>,
        from_network_message_tx: UnboundedSender<FromNetworkMessage>,
        to_network_message_rx: UnboundedReceiver<ToNetworkMessage>,
        server: Arc<Server>,
    ) {
        self.network_airport_handle = Some(tokio::spawn(Self::network_airport(
            from_network_message_rx,
            fs_interface,
        )));
        self.peer_broadcast_handle = Some(tokio::spawn(Self::contact_peers(
            self.peers.clone(),
            to_network_message_rx,
        )));
        self.new_peer_handle = Some(tokio::spawn(Self::incoming_connections_watchdog(
            server,
            from_network_message_tx.clone(),
            self.peers.clone(),
        )));
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
    /// Get a new inode, add the requested entry to the arbo and inform the network
    pub fn register_new_file(&self, inode: Inode) -> io::Result<u64> {
        let new_inode_id = inode.id;

        if let Some(mut arbo) = self.arbo.try_write_for(LOCK_TIMEOUT) {
            arbo.add_inode(new_inode_id, inode.clone())?;
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
            match arbo.add_inode(id, inode) {
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

    async fn network_airport(
        mut network_reception: UnboundedReceiver<FromNetworkMessage>,
        fs_interface: Arc<FsInterface>,
    ) {
        loop {
            let FromNetworkMessage { origin, content } = match network_reception.recv().await {
                Some(message) => message,
                None => continue,
            };

            match content {
                MessageContent::Binary(bin) => {
                    println!("peer: {:?}", String::from_utf8(bin).unwrap_or_default());
                }
                MessageContent::Inode(inode, id) => {
                    fs_interface.recept_inode(inode, id);
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

    async fn contact_peers(
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

    async fn incoming_connections_watchdog(
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