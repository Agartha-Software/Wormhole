use std::{collections::HashMap, net::SocketAddr, path::PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    cli::ConfigType,
    data::tree_hosts::CliHostTree,
    ipc::error::IoError,
    pods::{disk_managers::DiskSizeInfo, itree::Hosts, network::redundancy::RedundancyStatus},
};

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
    Success(String),
    PodNotFound,
    AlreadyUnfrozen,
    CouldntBind(IoError),
    PodCreationFailed(IoError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FreezeAnswer {
    Success(String),
    PodNotFound,
    AlreadyFrozen,
    PodBlock,
    PodStopFailed(IoError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RestartAnswer {
    Success(String),
    PodNotFound,
    PodFrozen,
    PodBlock,
    PodStopFailed(IoError),
    CouldntBind(IoError),
    PodCreationFailed(IoError),
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RemoveAnswer {
    Success,
    PodNotFound,
    PodStopFailed(IoError),
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

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ListPodsAnswer {
    Pods(Vec<String>),
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
    pub frozen: bool,
    pub public_url: Option<String>,
    pub bound_socket: SocketAddr,
    pub hostname: String,
    pub name: String,
    pub connected_peers: Vec<PeerInfo>,
    pub mount: PathBuf,
    pub disk_space: Option<DiskSizeInfo>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum InspectAnswer {
    Information(InspectInfo),
    PodNotFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TreeAnswer {
    Tree(CliHostTree),
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
pub enum RedundancyStatusAnswer {
    // Status(<RedundancyStatus, total_of_files_for_this_status>)
    Status(HashMap<RedundancyStatus, u64>),
    PodNotFound,
    InternalError,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum StatsPerFiletypeAnswer {
    Stats(HashMap<String, u64>),
    PodNotFound,
    InternalError,
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