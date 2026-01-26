use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::{GenericFilePath, Name, NameType, ToFsName, ToNsName};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions};
use std::io;

use crate::ipc::error::SocketListenerError;

pub static SOCKET_DEFAULT_NAME: &str = "wormhole.sock";

fn name_from_string<'n>(name: &String) -> Result<Name<'n>, SocketListenerError> {
    if GenericNamespaced::is_supported() {
        name.clone().to_ns_name::<GenericNamespaced>()
    } else {
        format!("/tmp/{name}").to_fs_name::<GenericFilePath>()
    }
    .map_err(|io| SocketListenerError::InvalidAddr { io })
}

pub fn new_socket_listener(
    specific_socket: Option<String>,
) -> Result<(Listener, String), SocketListenerError> {
    let name = specific_socket.unwrap_or(SOCKET_DEFAULT_NAME.to_string());
    let ns_name = name_from_string(&name)?;
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
