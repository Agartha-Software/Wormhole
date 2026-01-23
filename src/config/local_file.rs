use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocalConfigFile {
    pub name: Option<String>,
    pub listen_address: Option<Multiaddr>,
    pub restart: Option<bool>,
}
