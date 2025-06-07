use custom_error::custom_error;
use std::{fmt, io};

use crate::pods::pod::PodInfoError;
use crate::pods::pod::PodStopError;
use bincode;

custom_error! {pub WhError
    InodeNotFound = "Entry not found",
    InodeIsNotADirectory = "Entry is not a directory",
    InodeIsADirectory = "Entry is a directory",
    DeadLock = "A DeadLock occured",
    NetworkDied{called_from: String} = "{called_from}: Unable to update modification on the network",
    WouldBlock{called_from: String} = "{called_from}: Unable to lock arbo",
}

impl WhError {
    pub fn to_libc(&self) -> i32 {
        match self {
            WhError::InodeNotFound => libc::ENOENT,
            WhError::InodeIsNotADirectory => libc::ENOTDIR,
            WhError::InodeIsADirectory => libc::EISDIR,
            WhError::DeadLock => libc::EDEADLOCK,
            WhError::NetworkDied { called_from: _ } => libc::ENETDOWN,
            WhError::WouldBlock { called_from: _ } => libc::EWOULDBLOCK,
        }
    }
}

pub type WhResult<T> = Result<T, WhError>;

custom_error! {pub CliError
    BoxError{arg: Box<dyn std::error::Error>} = "{arg}",
    BincodeError = "Serialization error",
    TungsteniteError = "WebSocket error",
    IoError{source: io::Error} = "I/O error: {source}", // Pour les erreurs fs::remove_dir_all, etc.

    PodNotFound = "Pod not found",
    PodInfoError{source: PodInfoError} = "{source}",
    PodStopError{source: PodStopError} = "{source}",
    WhError{source: WhError} = "{source}",

    FileConfigName{name: String} = "This isn't a valid configuration's file: {name}",

    PodCreationFailed{reason: io::Error} = "Pod creation failed: {reason}",
    PodRemovalFailed{name: String} = "Pod removal failed, a pod with this name {name} doens't exist",

    InvalidConfig{file: String} = "Configuration file {file} is missing or invalid",
    InvalidCommand = "Unrecognized command",
    InvalidArgument{arg: String} = "Invalid Argument: {arg} is not recognized",

    Unimplemented{arg: String} = "{arg} not implemented",
    Server{addr: String, err: std::io::Error} = "Impossible to bind to this address {addr}",
    Message{reason: String} = "{reason}",
}

#[derive(Debug)]
pub enum CliSuccess {
    /// Succès avec un simple message
    Message(String),
    /// Succès avec un message et des données supplémentaires
    WithData { message: String, data: String },
}

impl fmt::Display for CliSuccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliSuccess::Message(msg) => write!(f, "{}", msg),
            CliSuccess::WithData { message, data } => {
                write!(f, "{} - Data:\n{}\n", message, data)
            }
        }
    }
}

impl From<Box<dyn std::error::Error>> for CliError {
    fn from(arg: Box<dyn std::error::Error>) -> Self {
        CliError::BoxError { arg }
    }
}

impl From<bincode::Error> for CliError {
    fn from(err: bincode::Error) -> Self {
        CliError::BoxError {
            arg: Box::new(err) as Box<dyn std::error::Error>,
        }
    }
}

// Conversion pour tungstenite::Error
impl From<tokio_tungstenite::tungstenite::Error> for CliError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        CliError::BoxError {
            arg: Box::new(err) as Box<dyn std::error::Error>,
        }
    }
}

pub type CliResult<T> = Result<T, CliError>;
