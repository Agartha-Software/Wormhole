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
use std::{env, path::PathBuf, sync::Arc};

use wormhole::pods::whpath::WhPath;
use wormhole::{network::server::Server, pods::declarations::Pod};

#[tokio::main]
async fn main() {
    let mut pods: Vec<Pod> = Vec::new();

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

    println!("own address: {}", own_addr);
    println!("peer1 address: {}", other_addr1);
    println!("peer2 address: {}", other_addr2);
    println!("\nstarting");

    let server = Arc::new(Server::setup(&own_addr).await);

    pods.push(
        Pod::new(
            WhPath::from(mount.as_path()),
            1,
            vec![other_addr1, other_addr2],
            server.clone(),
            own_addr,
        )
        .await
        .expect("failed to create the pod"),
    );

    let local_cli_handle = tokio::spawn(local_cli_watchdog());
    println!("started");
    local_cli_handle.await.unwrap(); // keeps the main process alive until interruption from this watchdog;
    println!("stopping");
}

// NOTE - old watchdog brought here for debug purposes
pub async fn local_cli_watchdog() {
    let mut stdin = tokio::io::stdin();
    let mut buf = vec![0; 1024];

    loop {
        let read = tokio::io::AsyncReadExt::read(&mut stdin, &mut buf).await;

        // NOTE -  on ctrl-D -> quit
        match read {
            Err(_) | Ok(0) => {
                println!("Quiting!");
                break;
            }
            _ => (),
        };
    }
}

/*
#[tokio::main]
async fn main2() {
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

    println!("own address: {}", own_addr);
    println!("peer1 address: {}", other_addr1);
    println!("peer2 address: {}", other_addr2);
    println!("\nstarting");

    let (nfa_tx, nfa_rx) = mpsc::unbounded_channel();
    let (local_fuse_tx, local_fuse_rx) = mpsc::unbounded_channel();
    let (_session, provider) = mount_fuse(&mount, local_fuse_tx.clone());

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

    request_filesystem(peers.clone());
    let peers_broadcast_handle = tokio::spawn(contact_peers(peers.clone(), local_fuse_rx));
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
