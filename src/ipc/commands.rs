use serde::{Deserialize, Serialize};

use crate::pods::whpath::WhPath;

#[derive(Debug, Serialize, Deserialize)]
pub enum StartRequest {
    Name(String),
    Path(WhPath),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Start(StartRequest),
}
