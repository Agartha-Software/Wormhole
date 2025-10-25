mod command;
mod socket;
mod tcp;

use crate::{
    ipc::{
        error::{ListenerError, TCPListenerError},
        service::{
            command::handle_connection, socket::new_socket_listener, tcp::new_free_tcp_listener,
        },
    },
    pods::pod::Pod,
};
use interprocess::local_socket::traits::tokio::Listener;
use std::collections::HashMap;
use tokio::{net::TcpListener, sync::mpsc::UnboundedReceiver};

/// Listens for CLI calls and launch one tcp instance per cli command
/// if `specific_ip` is not given, will try all ports starting from 8081 to 9999, incrementing until success
/// if `specific_ip` is given, will try the given ip and fail on error.
pub async fn start_commands_listeners(
    pods: &mut HashMap<String, Pod>,
    specific_ip: Option<String>,
    mut signals_rx: UnboundedReceiver<()>,
) -> Result<(), ListenerError> {
    let (tcp_listener, ip) = match specific_ip {
        Some(ip) => (
            TcpListener::bind(&ip).await.map_err(|err| {
                TCPListenerError::ProvidedIpNotAvailable {
                    ip: ip.to_string(),
                    err,
                }
            })?,
            ip,
        ),
        None => new_free_tcp_listener().await?,
    };
    log::info!("Started Tcp Listener on {}", ip.to_string());

    let socket_name = "wormhole.sock";
    let socket_listener = new_socket_listener(socket_name)?;
    log::info!("Started Socket Listener on {}", socket_name);

    loop {
        if tokio::select! {
            Ok((stream, _)) = tcp_listener.accept() => handle_connection(pods, stream).await?,
            Ok(stream) = socket_listener.accept() => handle_connection(pods, stream).await?,
            _ = signals_rx.recv() => true,
        } {
            return Ok(());
        };
    }
}
