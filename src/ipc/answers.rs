use std::path::PathBuf;

use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{cli::ConfigType, ipc::error::IoError, pods::itree::Hosts};

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum PodCreationError {
    DiskAccessError(IoError),
    ITreeIndexion(IoError),
    Mount(IoError),
    TransportError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NewAnswer {
    Success(Multiaddr, bool),
    AlreadyExist,
    AlreadyMounted,
    InvalidIp(String),
    PortAlreadyTaken,
    NoFreePortInRage,
    ConflictWithConfig(String),
    FailedToCreatePod(PodCreationError),
}

#[derive(Debug, Serialize, Deserialize)]
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
    Success(String),
    PodNotFound,
    AlreadyUnfrozen,
    CouldntBind(IoError),
    PodCreationFailed(PodCreationError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FreezeAnswer {
    Success(String),
    PodNotFound,
    AlreadyFrozen,
    PodBlock,
    PodStopFailed(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RestartAnswer {
    Success(String),
    PodNotFound,
    PodFrozen,
    PodBlock,
    PodStopFailed(String),
    CouldntBind(IoError),
    PodCreationFailed(PodCreationError),
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
pub struct StatusSuccess {
    pub running: Vec<String>,
    pub frozen: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum StatusAnswer {
    Success(StatusSuccess),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub name: String,
    pub listen_addrs: Vec<Multiaddr>,
}

impl std::fmt::Display for PeerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: \"{}\", Addresses: [ {} ]",
            self.name,
            self.listen_addrs
                .iter()
                .map(|address| address.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InspectInfo {
    pub frozen: bool,
    pub listen_addrs: Vec<Multiaddr>,
    pub name: String,
    pub connected_peers: Vec<PeerInfo>,
    pub mount: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
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
