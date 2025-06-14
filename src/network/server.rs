use super::message::ToNetworkMessage;
use crate::error::{CliError, CliResult};
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

pub struct Server {
    pub listener: TcpListener,
    pub state: PeerMap,
}

impl Server {
    pub async fn setup(addr: &str) -> CliResult<Server> {
        let socket_addr: SocketAddr = addr.parse().map_err(|e| CliError::Server {
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
