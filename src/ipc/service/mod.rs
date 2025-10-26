mod commands;
mod connection;
mod socket;
mod tcp;

use crate::{
    ipc::{
        error::{ListenerError, TCPListenerError},
        service::{
            connection::handle_connection, socket::new_socket_listener, tcp::new_free_tcp_listener,
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
    specific_socket: Option<String>,
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
    println!("Started Tcp Listener at '{}'", ip.to_string());

    let socket_name = specific_socket.unwrap_or("wormhole.sock".to_string());
    let socket_listener = new_socket_listener(&socket_name)?;
    println!("Started Socket Listener at '{}'", socket_name);
    println!("Wormhole running!");

    loop {
        if tokio::select! {
            Ok((stream, _)) = tcp_listener.accept() => handle_connection(pods, stream).await,
            Ok(stream) = socket_listener.accept() => handle_connection(pods, stream).await,
            _ = signals_rx.recv() => true,
        } {
            return Ok(());
        };
    }
}
