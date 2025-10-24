use serde::{Deserialize, Serialize};

pub mod cli;
pub mod error;
pub mod service;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandAnswer {
    Success,
    Failure,
}
