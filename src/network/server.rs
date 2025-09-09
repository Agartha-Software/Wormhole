use super::message::ToNetworkMessage;
use crate::{
    error::{CliError, CliResult, PortError},
    pods:pod::Pod,
    network::ip::MAX_PORT,
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
    pub async fn setup(addr: &str) -> CliResult<Server> {
        let socket_addr: SocketAddr = addr.parse().map_err(|_| PortError::AddressParseError {
            address: addr.to_string(),
        })?;

        let port = socket_addr.port();
        if port < 1024 || port > MAX_PORT {
            return Err(CliError::PortError {
                source: PortError::InvalidPort { port: port },
            });
        }

        let socket = TcpSocket::new_v4().map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;
        socket.set_reuseaddr(false).map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;

        match socket.bind(socket_addr) {
            Ok(_listener) => (),
            Err(e) => match e.kind() {
                std::io::ErrorKind::AddrInUse => {
                    return Err(PortError::PortAlreadyInUse {
                        port: socket_addr.port(),
                        address: addr.to_string(),
                    }
                    .into())
                }
                std::io::ErrorKind::PermissionDenied => {
                    return Err(PortError::InvalidPort {
                        port: socket_addr.port(),
                    }
                    .into())
                }
                _ => {
                    return Err(PortError::PortBindFailed {
                        address: addr.to_string(),
                        source: e,
                    }
                    .into())
                }
            },
        }

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
