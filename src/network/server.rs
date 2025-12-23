use super::message::ToNetworkMessage;
use crate::pods::pod::Pod;
use std::{
    collections::HashMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
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

pub const POD_DEFAULT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)); // Change to 127.0.0.0?
pub const POD_DEFAULT_PORT: u16 = 40000;
pub const POD_PORT_MAX_TRIES: u16 = 100;
pub const POD_PORT_RANGE_END: u16 = POD_DEFAULT_PORT + POD_PORT_MAX_TRIES;

fn new_tcp_socket() -> io::Result<TcpSocket> {
    let socket = TcpSocket::new_v4()
        .inspect_err(|e| log::error!("Failed to create the socket listener: {e}"))?;
    socket
        .set_reuseaddr(false)
        .inspect_err(|e| log::error!("Can't set reuseaddr of the new socket: {e}"))?;
    Ok(socket)
}

fn create_listener(socket: TcpSocket) -> io::Result<TcpListener> {
    socket
        .listen(1024)
        .inspect_err(|e| log::error!("Failed to create listener for the new pod: {e}"))
}

impl Server {
    pub fn new(ip_address: Option<IpAddr>, port: Option<u16>) -> io::Result<(Server, SocketAddr)> {
        let ip = ip_address.unwrap_or(POD_DEFAULT_IP.to_owned());

        match port {
            Some(port) => {
                let socket_addr = SocketAddr::new(ip, port);
                Server::from_specific_address(socket_addr).map(|server| (server, socket_addr))
            }
            None => Server::from_range(ip),
        }
    }

    pub fn from_specific_address(socket_addr: SocketAddr) -> io::Result<Server> {
        let socket = new_tcp_socket()?;

        socket
            .bind(socket_addr)
            .inspect_err(|e| log::trace!("Given socket address couldn't be bound: {e}"))?;

        Ok(Server {
            listener: create_listener(socket)?,
            state: PeerMap::new(Mutex::new(HashMap::new())),
        })
    }

    fn from_range(ip: IpAddr) -> io::Result<(Server, SocketAddr)> {
        let socket = new_tcp_socket()?;

        for port in POD_DEFAULT_PORT..POD_PORT_RANGE_END {
            let socket_addr = SocketAddr::new(ip, port);
            match socket.bind(socket_addr) {
                Ok(()) => {
                    return Ok((
                        Server {
                            listener: create_listener(socket)?,
                            state: PeerMap::new(Mutex::new(HashMap::new())),
                        },
                        socket_addr,
                    ))
                }
                Err(err) => {
                    log::trace!(
                        "Couldn't bind automatically generated address '{socket_addr}' '{err}', retrying!"
                    );
                    continue;
                }
            }
        }

        Err(
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("No valid address found in the range {ip}:[{POD_DEFAULT_PORT}..{POD_PORT_RANGE_END}]"),
            )
        )
    }
}
