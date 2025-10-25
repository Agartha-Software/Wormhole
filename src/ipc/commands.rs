use serde::{Deserialize, Serialize};

use crate::pods::whpath::WhPath;

#[derive(Debug, Serialize, Deserialize)]
pub enum PodId {
    Name(String),
    Path(WhPath),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UnfreezeAnswer {
    Success,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NewRequest {
    Name(String),
    Path(WhPath),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NewAnswer {
    Success,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Unfreeze(PodId),
    New(PodId),
}
