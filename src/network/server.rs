use super::message::ToNetworkMessage;
use crate::{ipc::answers::NewAnswer, pods::pod::Pod};
use std::{
    collections::HashMap,
    io,
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

pub const POD_DEFAULT_IP: &'static str = "0.0.0.0"; // Change to 127.0.0.0?
pub const POD_DEFAULT_PORT: u16 = 40000;
pub const POD_PORT_MAX_TRIES: u16 = 100;
pub const POD_PORT_RANGE_END: u16 = POD_DEFAULT_PORT + POD_PORT_MAX_TRIES;

fn connect_bind(socket: &TcpSocket, addr: String) -> Result<String, NewAnswer> {
    let socket_addr = addr.parse().map_err(|_| NewAnswer::InvalidIp)?;
    socket.bind(socket_addr).map_err(|e| {
        log::trace!("Automatically generated address is invalid, retrying: {e}");
        NewAnswer::BindImpossible(e.into())
    })?;
    Ok(addr)
}

fn new_tcp_socket() -> Result<TcpSocket, NewAnswer> {
    let socket = TcpSocket::new_v4().map_err(|e| {
        log::error!("Failed to bind new pod listener: {e}");
        NewAnswer::BindImpossible(e.into())
    })?;
    socket.set_reuseaddr(false).map_err(|e| {
        log::error!("Failed to bind new pod listener: {e}");
        NewAnswer::BindImpossible(e.into())
    })?;
    Ok(socket)
}

fn create_listener(socket: TcpSocket) -> Result<TcpListener, NewAnswer> {
    socket.listen(1024).map_err(|e| {
        log::error!("Failed to bind new pod listener: {e}");
        NewAnswer::BindImpossible(e.into())
    })
}

impl Server {
    pub fn from_ip_address(
        ip_address: Option<String>,
        port: Option<u16>,
    ) -> Result<(Server, String), NewAnswer> {
        let ip = ip_address.unwrap_or(POD_DEFAULT_IP.to_owned());

        match port {
            Some(port) => Server::from_socket_address(format!("{ip}:{port}")),
            None => Server::from_range(ip),
        }
    }

    pub fn from_socket_address(socket_address: String) -> Result<(Server, String), NewAnswer> {
        let socket = new_tcp_socket()?;

        let socket_address = connect_bind(&socket, socket_address)?;

        Ok((
            Server {
                listener: create_listener(socket)?,
                state: PeerMap::new(Mutex::new(HashMap::new())),
            },
            socket_address,
        ))
    }

    fn from_range(ip: String) -> Result<(Server, String), NewAnswer> {
        let socket = new_tcp_socket()?;

        let socket_address = (POD_DEFAULT_PORT..POD_PORT_RANGE_END)
            .find_map(|port| connect_bind(&socket, format!("{ip}:{port}")).ok())
            .ok_or(NewAnswer::BindImpossible(
                io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    format!("No valid address found in the range {ip}:[{POD_DEFAULT_PORT}..{POD_PORT_RANGE_END}]"),
                )
                .into(),
            ))?;

        Ok((
            Server {
                listener: create_listener(socket)?,
                state: PeerMap::new(Mutex::new(HashMap::new())),
            },
            socket_address,
        ))
    }
}
