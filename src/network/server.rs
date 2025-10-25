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

pub const POD_DEFAULT_PORT: u16 = 40000;
pub const POD_PORT_MAX_TRIES: u16 = 100;

fn connect_bind(socket: &TcpSocket, addr: String) -> CliResult<()> {
    let socket_addr = addr.parse().map_err(|_| CliError::Server {
        addr: addr.to_owned(),
        err: std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid ip address"),
    })?;
    socket.bind(socket_addr).map_err(|e| CliError::Server {
        addr: addr.to_owned(),
        err: e,
    })
}

fn connect_to_available_port(
    socket: &TcpSocket,
    addr: &str,
    port: Option<String>,
) -> CliResult<String> {
    if let Some(port) = port {
        let combined = format!("{}:{}", addr, port);
        connect_bind(socket, combined)?;
        return Ok(port);
    }

    let mut last_err: Option<CliError> = None;
    for p in 0..POD_PORT_MAX_TRIES {
        let port_num = POD_DEFAULT_PORT + p;
        let combined = format!("{}:{}", addr, port_num);
        match connect_bind(socket, combined) {
            Ok(()) => return Ok(port_num.to_string()),
            Err(e) => last_err = Some(e),
        }
    }

    // NOTE technically impossible to go there
    Err(last_err.unwrap_or_else(|| CliError::Server {
        addr: addr.to_owned(),
        err: std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "no available port in range",
        ),
    }))
}

impl Server {
    pub async fn setup(addr: &str, port: Option<String>) -> CliResult<(Server, String)> {
        let socket = TcpSocket::new_v4().map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;
        socket.set_reuseaddr(false).map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;
        let port = connect_to_available_port(&socket, addr, port)?;
        let listener = socket.listen(1024).map_err(|e| CliError::Server {
            addr: addr.to_owned(),
            err: e,
        })?;

        Ok((
            Server {
                listener: listener,
                state: PeerMap::new(Mutex::new(HashMap::new())),
            },
            port,
        ))
    }
}
