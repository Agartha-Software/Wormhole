use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{ipc::error::IoError, pods::arbo::Hosts};

#[derive(Debug, Serialize, Deserialize)]
pub enum NewAnswer {
    Success(u16),
    AlreadyExist,
    InvalidIp,
    BindImpossible(IoError),
    FailedToCreatePod(IoError),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GetHostsAnswer {
    Hosts(Hosts),
    FileNotInsideARunningPod,
    FileNotFound,
    WrongFileType(String),
    FailedToGetHosts(IoError),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UnfreezeAnswer {
    Success,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FreezeAnswer {
    Success,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RemoveAnswer {
    Success,
    PodNotFound,
    PodStopFailed(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub hostname: String,
    pub url: Option<String>,
}

impl std::fmt::Display for PeerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "hostname: {}, url: {}",
            self.hostname,
            self.url
                .as_ref()
                .map(|url| url.as_str())
                .unwrap_or("Undefined")
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InspectInfo {
    pub url: Option<String>,
    pub hostname: String,
    pub name: String,
    pub connected_peers: Vec<PeerInfo>,
    pub mount: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum InspectAnswer {
    Information(InspectInfo),
    PodNotFound,
}
