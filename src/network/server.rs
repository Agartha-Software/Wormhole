use super::message::ToNetworkMessage;
use crate::{
    error::{CliError, CliResult},
    ipc::commands::NewAnswer,
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
pub type Tx = UnboundedReceiver<ToNetworkMessage>;
pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct Service {
    // pub server: Server
    pub pods: HashMap<String, Pod>,
}

pub struct Server {
    pub listener: TcpListener,
    pub state: PeerMap,
}

impl Server {
    pub async fn setup(addr: &str) -> Result<Server, NewAnswer> {
        let socket_addr: SocketAddr = addr.parse().map_err(|_| NewAnswer::InvalidIp)?;

        let socket = TcpSocket::new_v4().map_err(|e| {
            log::error!("Failed to bind new pod listener: {e}");
            NewAnswer::BindImpossible(e.to_string())
        })?;
        socket.set_reuseaddr(false).map_err(|e| {
            log::error!("Failed to bind new pod listener: {e}");
            NewAnswer::BindImpossible(e.to_string())
        })?;
        socket.bind(socket_addr).map_err(|e| {
            log::error!("Failed to bind new pod listener: {e}");
            NewAnswer::BindImpossible(e.to_string())
        })?;
        let listener = socket.listen(1024).map_err(|e| {
            log::error!("Failed to bind new pod listener: {e}");
            NewAnswer::BindImpossible(e.to_string())
        })?;

        Ok(Server {
            listener: listener,
            state: PeerMap::new(Mutex::new(HashMap::new())),
        })
    }
}
