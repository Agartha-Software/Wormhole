use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum PodId {
    Name(String),
    Path(PathBuf),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UnfreezeAnswer {
    Success,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewRequest {
    pub name: String,
    pub port: u16,
    pub mountpoint: PathBuf,
    pub url: Option<String>,
    pub hostname: Option<String>,
    pub listen_url: Option<String>,
    pub additional_hosts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NewAnswer {
    Success,
    AlreadyExist,
    InvalidIp,
    BindImpossible,
    NoSpecifiedPeersHaveAnswerd,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Unfreeze(PodId),
    New(NewRequest),
}
