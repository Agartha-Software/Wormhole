use super::message::ToNetworkMessage;
use crate::{
    error::{CliError, CliResult},
    pods::pod::Pod,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    net::{TcpListener, TcpSocket},
    sync::mpsc::UnboundedReceiver,
};

/// Receiver channel for network-bound messages.
/// Each peer connection will have its own channel.
pub type Tx = UnboundedReceiver<ToNetworkMessage>;

/// Thread-safe map of active peer connections.
/// Maps SocketAddr to the corresponding Tx channel.
/// Uses Arc and Mutex for safe concurrent access across threads.
pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct Service {
    // pub server: Server
    pub pods: HashMap<String, Pod>,
}

/// WebSocket server managing incoming peer connections.
///
/// The `Server` serves as the main entry point for new peers joining the network.
/// It handles TCP connection acceptance and maintains shared state between all connected peers.
pub struct Server {
    /// TCP listener for accepting incoming peer connections.
    /// Listens on the specified address and port. Used in the main event loop to accept new connections.
    pub listener: TcpListener,

    /// Shared state containing active peer connections.
    ///
    /// Maps each peer's SocketAddr to its corresponding Tx channel for sending messages.
    /// Wrapped in Arc and Mutex for safe concurrent access across multiple threads.
    pub state: PeerMap,
}

impl Server {
    /// Configures and initializes a new TCP server.
    /// Creates a complete TCP server ready to accept connections on the specified address.
    pub async fn setup(addr: &str) -> CliResult<Server> {
        let socket_addr: SocketAddr = addr.parse().map_err(|_| CliError::Server {
            addr: addr.to_owned(),
            err: std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid ip address"),
        })?;

        let socket = TcpSocket::new_v4().map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;
        socket.set_reuseaddr(false).map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;
        socket.bind(socket_addr).map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;
        let listener = socket.listen(1024).map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;

        Ok(Server {
            listener: listener,
            state: PeerMap::new(Mutex::new(HashMap::new())),
        })
    }
}
