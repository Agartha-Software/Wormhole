use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocalConfigFile {
    pub name: Option<String>,
    pub listen_addrs: Vec<String>,
    pub restart: Option<bool>,
}
