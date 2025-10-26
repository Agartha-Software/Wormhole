use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::cli::IdentifyPodArgs;

#[derive(Debug, Serialize, Deserialize)]
pub enum PodId {
    Name(String),
    Path(PathBuf),
}

impl From<IdentifyPodArgs> for PodId {
    fn from(args: IdentifyPodArgs) -> Self {
        if let Some(name) = args.group.name {
            PodId::Name(name)
        } else {
            if let Some(path) = args.group.path {
                PodId::Path(path)
            } else {
                panic!("One of path or name should always be defined, if both are missing Clap should block the cmd")
            }
        }
    }
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
pub struct GetHostsRequest {
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Unfreeze(PodId),
    Freeze(PodId),
    New(NewRequest),
    GetHosts(GetHostsRequest),
    Inspect(PodId),
}
