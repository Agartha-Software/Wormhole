use custom_error::custom_error;
use std::io;

custom_error! {
    #[derive(Clone)]
    pub WhError
    InodeNotFound = "Entry not found",
    InodeIsNotADirectory = "Entry is not a directory",
    InodeIsADirectory = "Entry is a directory",
    DeadLock = "A DeadLock occured",
    NetworkDied{called_from: String} = "{called_from}: Unable to update modification on the network",
    WouldBlock{called_from: String} = "{called_from}: Unable to lock itree",
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

impl From<WhError> for io::ErrorKind {
    fn from(value: WhError) -> Self {
        match value {
            WhError::InodeNotFound => io::ErrorKind::NotFound,
            WhError::InodeIsNotADirectory => io::ErrorKind::NotADirectory,
            WhError::InodeIsADirectory => io::ErrorKind::IsADirectory,
            WhError::DeadLock => io::ErrorKind::Deadlock,
            WhError::NetworkDied { called_from: _ } => io::ErrorKind::NetworkDown,
            WhError::WouldBlock { called_from: _ } => io::ErrorKind::WouldBlock,
        }
    }
}

impl From<WhError> for io::Error {
    fn from(value: WhError) -> Self {
        io::Error::new(value.clone().into(), value.to_string())
    }
}

pub type WhResult<T> = Result<T, WhError>;
