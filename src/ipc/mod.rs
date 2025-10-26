use serde::{Deserialize, Serialize};

pub mod answers;
pub mod cli;
pub mod commands;
pub mod error;
pub mod service;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandAnswer {
    Success,
    Failure,
}
