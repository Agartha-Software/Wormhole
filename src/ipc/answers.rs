use std::{net::SocketAddr, path::PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{cli::ConfigType, ipc::error::IoError, pods::itree::Hosts};

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum NewAnswer {
    Success(SocketAddr),
    AlreadyExist,
    AlreadyMounted,
    InvalidIp,
    InvalidUrlIp,
    ConflictWithConfig(String),
    BindImpossible(IoError),
    FailedToCreatePod(IoError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum GetHostsAnswer {
    Hosts(Hosts),
    FileNotInsideARunningPod,
    FileNotFound,
    WrongFileType(String),
    FailedToGetHosts(IoError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum UnfreezeAnswer {
    Success,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FreezeAnswer {
    Success,
    PodNotFound,
    AlreadyFrozen,
    PodBlock,
    PodStopFailed(String),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RemoveAnswer {
    Success,
    PodNotFound,
    PodStopFailed(String),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum StatusAnswer {
    Success,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PeerInfo {
    pub hostname: String,
    pub url: Option<String>,
}

impl std::fmt::Display for PeerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Hostname: \"{}\", Url: {}",
            self.hostname,
            self.url.clone().unwrap_or("None".to_string())
        )
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InspectInfo {
    pub public_url: Option<String>,
    pub bound_socket: SocketAddr,
    pub hostname: String,
    pub name: String,
    pub connected_peers: Vec<PeerInfo>,
    pub mount: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum InspectAnswer {
    Information(InspectInfo),
    PodNotFound,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum TreeAnswer {
    Tree(String),
    PodNotFound,
    PodTreeFailed(IoError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum GenerateConfigAnswer {
    Success,
    SuccessDefault,
    PodNotFound,
    NotADirectory,
    WriteFailed(String, ConfigType),
    CantOverwrite(ConfigType),
    ConfigBlock,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ShowConfigAnswer {
    SuccessBoth(String, String),
    SuccessLocal(String),
    SuccessGlobal(String),
    PodNotFound,
    ConfigBlock,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ConfigFileError {
    MissingGlobal,
    MissingLocal,
    MissingBoth,
    InvalidGlobal(String),
    InvalidLocal(String),
    InvalidBoth(String, String),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum CheckConfigAnswer {
    Success,
    PodNotFound,
    ConfigFileError(ConfigFileError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ApplyConfigAnswer {
    Success,
    PodNotFound,
    ConfigFileError(ConfigFileError),
}
