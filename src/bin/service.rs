// In rust we code
// In code we trust
// AgarthaSoftware - 2024

/**DOC
 * Important variables to know :
 * nfa_rx - nfa_tx
 *  Use nfa_tx to send a file related message to the newtork_file_actions function
 *
 * Important functions to know :
 *
 * local_cli_watchdog
 *  this is the handle linked to the terminal, that will terminate the
 *  program if CTRL-D
 *
 * newtork_file_actions
 *  reads a message (supposely emitted by a peer) related to files actions
 *  and execute instructions on the disk
 */
use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    sync::mpsc::{self, UnboundedReceiver},
    task::JoinHandle,
};

use wormhole::network::{
    peers_operations::{all_peers_broadcast, peer_startup},
    watchdogs::{incoming_connections_watchdog, local_cli_watchdog, network_file_actions},
};
use wormhole::{fuse::fuse_impl::mount_fuse, network::peer_ipc::PeerIPC};

use wormhole::network::server::Server;

pub enum MsgType {
    Increment,
    Decrement,
}
struct Test {
    pub number: u64,
}

impl Test {
    pub async fn increment(nb: &mut u64) {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        *nb += 1;
    }

    pub async fn decrement(nb: &mut u64) {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        *nb -= 1;
    }

    pub async fn airport_loop(
        mut rx1: UnboundedReceiver<MsgType>,
        mut rx2: UnboundedReceiver<MsgType>,
        nb: &mut u64,
    ) {
        loop {
            tokio::select! {
                _a = rx1.recv() => Self::increment(nb).await,
                _a = rx2.recv() => Self::decrement(nb).await,
            }
        }
    }
    pub async fn airport(
        &mut self,
        mut rx1: UnboundedReceiver<MsgType>,
        mut rx2: UnboundedReceiver<MsgType>,
    ) -> JoinHandle<()> {
        tokio::spawn(Self::airport_loop(rx1, rx2, &mut self.number))
    }
}

#[tokio::main]
async fn main() {
    let (increment_tx, increment_rx) = mpsc::unbounded_channel::<MsgType>();
    let (decrement_tx, decrement_rx) = mpsc::unbounded_channel::<MsgType>();
    let mut test = Test { number: 10 };
}

/*
    env_logger::init();

    // DOC - arguments: own_address other_addr1 other_addr2 mount_to source
    let own_addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let other_addr1 = env::args()
        .nth(2)
        .unwrap_or("ws://127.0.0.2:8080".to_string());
    let other_addr2 = env::args()
        .nth(3)
        .unwrap_or("ws://127.0.0.3:8080".to_string());
    let mount: PathBuf = env::args()
        .nth(4)
        .unwrap_or("./virtual/".to_string())
        .into();
    let source: PathBuf = env::args()
        .nth(5)
        .unwrap_or("./original/".to_string())
        .into();

    println!("own address: {}", own_addr);
    println!("peer1 address: {}", other_addr1);
    println!("peer2 address: {}", other_addr2);

    println!("\nstarting");

    let (nfa_tx, nfa_rx) = mpsc::unbounded_channel();
    let (local_fuse_tx, local_fuse_rx) = mpsc::unbounded_channel();
    let (_session, provider) = mount_fuse(&source, &mount, local_fuse_tx.clone());

    let local_cli_handle = tokio::spawn(local_cli_watchdog());
    let nfa_handle = tokio::spawn(network_file_actions(nfa_rx, provider));
    let server = Server::setup(&own_addr).await;

    let peers = peer_startup(vec![other_addr1, other_addr2], nfa_tx.clone()).await;
    println!(
        "successful peers at startup :\n{:?}",
        peers
            .iter()
            .map(|p| p.address.clone())
            .collect::<Vec<String>>()
    );
    let peers: Arc<Mutex<Vec<PeerIPC>>> = Arc::new(Mutex::new(peers));

    let new_conn_handle = tokio::spawn(incoming_connections_watchdog(
        server,
        nfa_tx.clone(),
        peers.clone(),
    ));

    let peers_broadcast_handle = tokio::spawn(all_peers_broadcast(peers.clone(), local_fuse_rx));
    // let remote_reception = tokio::spawn(all_peers_reception(connected_peers, nfa_tx));

    println!("started");
    local_cli_handle.await.unwrap(); // keeps the main process alive until interruption from this watchdog;
    println!("stopping");
    new_conn_handle.abort();
    peers.lock().unwrap().iter().for_each(|peer| {
        peer.thread.abort();
    });
    nfa_handle.abort();
    peers_broadcast_handle.abort();
    println!("stopped");

}

*/
