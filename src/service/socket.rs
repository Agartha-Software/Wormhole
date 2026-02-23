use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::{ListenerOptions, Name};
use std::io;

use crate::ipc::error::SocketListenerError;

#[cfg(target_os = "linux")]
use interprocess::local_socket::{GenericFilePath, ToFsName};

#[cfg(target_os = "windows")]
use interprocess::local_socket::{GenericNamespaced, ToNsName};

#[cfg(target_os = "linux")]
pub static SOCKET_DEFAULT_NAME: &str = "/tmp/wormhole.sock";

#[cfg(target_os = "windows")]
pub static SOCKET_DEFAULT_NAME: &str = "wormhole.sock";

pub fn name_from_string<'n>(name: &str) -> io::Result<Name<'n>> {
    #[cfg(target_os = "linux")]
    {
        name.to_owned().to_fs_name::<GenericFilePath>()
    }
    #[cfg(target_os = "windows")]
    {
        name.to_owned().to_ns_name::<GenericNamespaced>()
    }
}

pub fn new_socket_listener(
    specific_socket: Option<String>,
) -> Result<(Listener, String), SocketListenerError> {
    let name = specific_socket.unwrap_or(SOCKET_DEFAULT_NAME.to_string());
    let ns_name = name_from_string(&name).map_err(|io| SocketListenerError::InvalidAddr { io })?;
    let listener = match ListenerOptions::new().name(ns_name).create_tokio() {
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            return Err(SocketListenerError::AddrInUse { name })
        }
        #[cfg(windows)]
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            return Err(SocketListenerError::AddrInUse { name })
        }
        Err(e) => panic!("Unhandled socket error during listener creation: {e}"),
        Ok(x) => x,
    };

    println!("Started Socket Listener at '{}'", name);
    Ok((listener, name))
}
