use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use tokio::{
    net::{unix::SocketAddr, TcpListener},
    sync::mpsc::UnboundedReceiver,
};

use crate::{config, data::metadata::MetaData};

use super::{message::NetworkMessage, peer_ipc::PeerIPC};

pub type Tx = UnboundedReceiver<NetworkMessage>;
pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct Server {
    pub listener: TcpListener,
    pub state: PeerMap,
}

impl Server {
    pub async fn setup(addr: &str) -> Server {
        Server {
            listener: TcpListener::bind(addr).await.expect("Failed to bind"),
            state: PeerMap::new(Mutex::new(HashMap::new())),
        }
    }
}

pub struct Pod {
    pub network: config::Network,
    pub directory: openat::Dir,
    pub path: std::path::PathBuf,
    pub name: String,
    pub metas: HashMap<String, MetaData>
    // fuser: !,
}

#[derive(Default)]
pub struct State {
    pub peers: RwLock<Vec<PeerIPC>>,
    pub pods: RwLock<HashMap<String, Pod>>,
}
