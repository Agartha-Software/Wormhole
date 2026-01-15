use custom_error::custom_error;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};
use ts_rs::TS;

custom_error! {pub ListenerError
    TCPListenerError { source: TCPListenerError } = "{source}",
    SocketListenerError { source: SocketListenerError } = "{source}",
}

custom_error! {pub TCPListenerError
    ProvidedIpNotAvailable {ip: String, err: std::io::Error} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMainPort {max_port: u16} = "Unable to start the TCP listener (not testing ports above {max_port})",
    AboveMaxTry {max_try_port: u16} = "Unable to start TCP listener (tested {max_try_port} ports)",
}

custom_error! {pub SocketListenerError
    AddrInUse { name: String } = "Could not start the server because the socket file is occupied. Please check\nif {name} is in use by another process and try again.",
    InvalidAddr { io: io::Error } = "The given socket address is invalid: {io}"
}

fn serialize<S>(kind: &ErrorKind, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u8(match kind {
        ErrorKind::NotFound => 1,
        ErrorKind::PermissionDenied => 2,
        ErrorKind::ConnectionRefused => 3,
        ErrorKind::ConnectionReset => 4,
        ErrorKind::ConnectionAborted => 5,
        ErrorKind::NotConnected => 6,
        ErrorKind::AddrInUse => 7,
        ErrorKind::AddrNotAvailable => 8,
        ErrorKind::BrokenPipe => 9,
        ErrorKind::AlreadyExists => 10,
        ErrorKind::WouldBlock => 11,
        ErrorKind::InvalidInput => 12,
        ErrorKind::InvalidData => 13,
        ErrorKind::TimedOut => 14,
        ErrorKind::WriteZero => 15,
        ErrorKind::Interrupted => 16,
        ErrorKind::Unsupported => 17,
        ErrorKind::UnexpectedEof => 18,
        ErrorKind::OutOfMemory => 19,
        ErrorKind::Other => 20,
        _ => 21,
    })
}

fn deserialize<'de, D>(deserializer: D) -> Result<ErrorKind, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let code = u8::deserialize(deserializer)?;
    Ok(match code {
        1 => ErrorKind::NotFound,
        2 => ErrorKind::PermissionDenied,
        3 => ErrorKind::ConnectionRefused,
        4 => ErrorKind::ConnectionReset,
        5 => ErrorKind::ConnectionAborted,
        6 => ErrorKind::NotConnected,
        7 => ErrorKind::AddrInUse,
        8 => ErrorKind::AddrNotAvailable,
        9 => ErrorKind::BrokenPipe,
        10 => ErrorKind::AlreadyExists,
        11 => ErrorKind::WouldBlock,
        12 => ErrorKind::InvalidInput,
        13 => ErrorKind::InvalidData,
        14 => ErrorKind::TimedOut,
        15 => ErrorKind::WriteZero,
        16 => ErrorKind::Interrupted,
        17 => ErrorKind::Unsupported,
        18 => ErrorKind::UnexpectedEof,
        19 => ErrorKind::OutOfMemory,
        20 => ErrorKind::Other,
        _ => ErrorKind::Other,
    })
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IoError {
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    #[ts(skip)]
    pub kind: ErrorKind,
    pub error: String,
}

impl From<std::io::Error> for IoError {
    fn from(value: std::io::Error) -> Self {
        IoError {
            kind: value.kind(),
            error: value.to_string(),
        }
    }
}

impl Into<std::io::Error> for IoError {
    fn into(self) -> std::io::Error {
        std::io::Error::new(self.kind, self.error)
    }
}
