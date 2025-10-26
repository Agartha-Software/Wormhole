use serde::{Deserialize, Serialize};

use crate::{ipc::error::IoError, pods::arbo::Hosts};

#[derive(Debug, Serialize, Deserialize)]
pub enum NewAnswer {
    Success,
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
