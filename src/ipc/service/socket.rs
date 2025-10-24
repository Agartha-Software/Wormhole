use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::ToNsName;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions};

use crate::ipc::error::SocketListenerError;

pub fn new_socket_listener(name: &'static str) -> Result<Listener, SocketListenerError> {
    let ns_name = name
        .to_ns_name::<GenericNamespaced>()
        .expect("Invalid socket file name, the name is static so it shouldn't happen.");

    let opts = ListenerOptions::new().name(ns_name);

    match opts.create_tokio() {
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            Err(SocketListenerError::AddrInUse { name })
        }
        Err(e) => panic!("Unhandled socket error during listener creation: {e}"),
        Ok(x) => Ok(x),
    }
}
