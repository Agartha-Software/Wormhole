use std::sync::{Arc, Mutex};

use futures_util::future::join_all;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::network::peer_ipc::PeerIPC;

use crate::network::message::NetworkMessage;

// receive a message on user_rx and broadcast it to all peers
pub async fn all_peers_broadcast(
    peers_list: Arc<Mutex<Vec<PeerIPC>>>,
    mut rx: UnboundedReceiver<NetworkMessage>,
) {
    // on message reception, broadcast it to all peers senders
    while let Some(message) = rx.recv().await {
        //generating peers senders
        // REVIEW - should avoid locking peers in future versions, as it more or less locks the entire program
        let peer_tx: Vec<(UnboundedSender<NetworkMessage>, String)> = peers_list
            .lock()
            .unwrap()
            .iter()
            .map(|peer| (peer.sender.clone(), peer.address.clone()))
            .collect();

        println!("broadcasting message to peers:\n{:?}", message);
        peer_tx.iter().for_each(|peer| {
            println!("peer: {}", peer.1);
            peer.0
                .send(message.clone())
                .expect(&format!("failed to send message to peer {}", peer.1))
        });
    }
}

// start connexions to peers
pub async fn peer_startup(
    peers_ip_list: Vec<String>,
    nfa_tx: UnboundedSender<NetworkMessage>,
) -> Vec<PeerIPC> {
    join_all(
        peers_ip_list
            .into_iter()
            .map(|ip| PeerIPC::connect(ip, nfa_tx.clone())), // .filter(|peer| !peer.thread.is_finished())
    )
    .await
    .into_iter()
    .flatten()
    .collect()
}
