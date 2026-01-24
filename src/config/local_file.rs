use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocalConfigFile {
    pub name: Option<String>,
    pub listen_addrs: Vec<Multiaddr>,
    pub restart: Option<bool>,
}
