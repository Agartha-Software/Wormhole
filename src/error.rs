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

    pub fn into_io(self) -> io::Error {
        match self {
            WhError::InodeNotFound => io::ErrorKind::NotFound.into(),
            WhError::InodeIsNotADirectory => io::ErrorKind::NotADirectory.into(),
            WhError::InodeIsADirectory => io::ErrorKind::IsADirectory.into(),
            other => io::Error::other(other),
        }
    }
}

pub type WhResult<T> = Result<T, WhError>;
