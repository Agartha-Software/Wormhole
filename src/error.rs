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

    pub fn into_io(self) -> io::Error {
        io::Error::other(self)
    }
}

pub type WhResult<T> = Result<T, WhError>;

custom_error! {pub CliError
    BoxError{arg: Box<dyn std::error::Error>} = "{arg}",
    BincodeError = "Serialization error",
    TungsteniteError = "WebSocket error",
    IoError{source: io::Error} = "I/O error: {source}", // Pour les erreurs fs::remove_dir_all, etc.
    PortError{source: PortError} = "{source}",

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

custom_error! { pub PortError
    InvalidPort{port: u16 } = "Invalid port: {port}. The port must be between 1024 and 65535",
    PortAlreadyInUse{port: u16,  address: String} = "Port {port} is already in use on {address}",
    PortBindFailed{address: String, source: std::io::Error} = "Unable to bind to address {address}: {source}",
    AddressParseError{address: String} = "Invalid address format: {address}. Expect format: IP:PORT",
}

custom_error! { pub CliListenerError
    ProvidedIpNotAvailable {ip: String, err: String} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMainPort {max_port: u16} = "Unable to start cli_listener (not testing ports above {max_port})",
    AboveMaxTry {max_try_port: u16} = "Unable to start cli_listener (tested {max_try_port} ports)",
}

#[derive(Debug)]
pub enum CliSuccess {
    /// Success with a simple message
    Message(String),
    /// Success with a message and additional data
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

// Conversion for tungstenite::Error
impl From<tokio_tungstenite::tungstenite::Error> for CliError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        CliError::BoxError {
            arg: Box::new(err) as Box<dyn std::error::Error>,
        }
    }
}

pub type CliResult<T> = Result<T, CliError>;
