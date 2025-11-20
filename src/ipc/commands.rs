use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::cli::{ConfigType, IdentifyNewPodGroup, IdentifyPodArgs, IdentifyPodGroup, Mode};

#[derive(Debug, Serialize, Deserialize)]
pub enum PodId {
    Name(String),
    Path(PathBuf),
}

impl From<IdentifyPodArgs> for PodId {
    fn from(args: IdentifyPodArgs) -> Self {
        PodId::from(args.group)
    }
}

impl From<IdentifyPodGroup> for PodId {
    fn from(group: IdentifyPodGroup) -> Self {
        if let Some(name) = group.name {
            PodId::Name(name)
        } else {
            PodId::Path(group.path.expect("One of path or name should always be defined, if both are missing Clap should block the cmd"))
        }
    }
}

impl From<IdentifyNewPodGroup> for PodId {
    fn from(group: IdentifyNewPodGroup) -> Self {
        if let Some(name) = group.name {
            PodId::Name(name)
        } else {
            PodId::Path(group.path.expect("One of path or name should always be defined, if both are missing Clap should block the cmd"))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewRequest {
    pub name: String,
    pub port: Option<u16>,
    pub mountpoint: PathBuf,
    pub url: Option<String>,
    pub hostname: Option<String>,
    pub listen_url: Option<String>,
    pub additional_hosts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetHostsRequest {
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveRequest {
    pub pod: PodId,
    pub mode: Mode,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Unfreeze(PodId),
    Remove(RemoveRequest),
    Freeze(PodId),
    New(NewRequest),
    GetHosts(GetHostsRequest),
    Inspect(PodId),
    Tree(PodId),
    SaveConfig(PodId, bool, ConfigType),
    ShowConfig(PodId, ConfigType),
    CheckConfig(PodId, ConfigType),
    Status,
}
