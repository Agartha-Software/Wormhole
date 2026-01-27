use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{cli::ConfigType, data::tree_hosts::TreeData, ipc::error::IoError, pods::itree::Hosts};

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
    /// Port, Dialed: has connected to an existing peer
    Success(String, bool),
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
    pub nickname: String,
    pub running: Vec<String>,
    pub frozen: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum StatusAnswer {
    Success(StatusSuccess),
}

/// Not to be confused with [PeerInfo](crate::network::PeerInfo)
/// though the two are the same data, this one is exclusively the CLI messaging
/// the other is exclusively for Network messaging
/// this distinction is because of typing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfoIPC {
    pub nickname: String,
    pub listen_addrs: Vec<String>,
}

impl From<&crate::network::PeerInfo> for PeerInfoIPC {
    fn from(value: &crate::network::PeerInfo) -> Self {
        value.to_ipc()
    }
}

impl std::fmt::Display for PeerInfoIPC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Nickname: \"{}\", Addresses: [ {} ]",
            self.nickname,
            self.listen_addrs.join(", ")
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InspectInfo {
    pub frozen: bool,
    pub listen_addrs: Vec<String>,
    pub name: String,
    pub connected_peers: Vec<PeerInfoIPC>,
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
    Tree(Box<TreeData>),
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
