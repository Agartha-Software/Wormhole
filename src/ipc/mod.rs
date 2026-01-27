use serde::{Deserialize, Serialize};

pub mod answers;
pub mod commands;
pub mod error;

pub use answers::PeerInfoIPC as PeerInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandAnswer {
    Success,
    Failure,
}
