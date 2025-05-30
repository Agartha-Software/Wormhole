use super::message::ToNetworkMessage;
use crate::error::{CliError, CliResult};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::mpsc::UnboundedReceiver};
pub type Tx = UnboundedReceiver<ToNetworkMessage>;
pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct Server {
    pub listener: TcpListener,
    pub state: PeerMap,
}

impl Server {
    pub async fn setup(addr: &str) -> CliResult<Server> {
        Ok(Server {
            listener: TcpListener::bind(addr)
                .await
                .map_err(|e| CliError::Server {
                    addr: addr.to_owned(),
                    err: e,
                })?,
            state: PeerMap::new(Mutex::new(HashMap::new())),
        })
    }
}
