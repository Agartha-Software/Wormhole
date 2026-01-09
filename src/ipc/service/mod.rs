mod commands;
mod connection;
mod socket;
mod tcp;

use crate::{
    ipc::{
        commands::Command,
        error::ListenerError,
        service::{
            connection::{handle_command, handle_connection},
            socket::new_socket_listener,
            tcp::{create_tcp_socket, handle_tcp_connection, rest_app_handler},
        },
    },
    pods::pod::Pod,
};
use interprocess::local_socket::traits::tokio::Listener;
use std::{collections::HashMap, future::IntoFuture};
use tokio::sync::{
    mpsc::{self, UnboundedReceiver},
    oneshot,
};

pub static SOCKET_DEFAULT_NAME: &str = "wormhole.sock";

/// Listens for CLI calls and launch one tcp instance per cli command
/// if `specific_ip` is not given, will try all ports starting from 8081 to 9999, incrementing until success
/// if `specific_ip` is given, will try the given ip and fail on error.
pub async fn start_commands_listeners(
    pods: &mut HashMap<String, Pod>,
    specific_ip: Option<String>,
    specific_socket: Option<String>,
    mut signals_rx: UnboundedReceiver<()>,
) -> Result<(), ListenerError> {
    // SECTION WEB API

    let (tcp_listener, ip) = create_tcp_socket(specific_ip).await?;
    println!("Started Tcp Listener at '{}'", ip.to_string());

    let (tx, mut rx) = mpsc::channel::<(Command, oneshot::Sender<String>)>(100);
    let rest_app = axum::Router::new()
        .route("/", axum::routing::get(rest_app_handler))
        .with_state(tx);
    let rest_service = tokio::spawn(axum::serve(tcp_listener, rest_app).into_future());

    // !SECTION

    // SECTION OS-PIPES

    let socket_name = specific_socket.unwrap_or(SOCKET_DEFAULT_NAME.to_string());
    let socket_listener = new_socket_listener(&socket_name)?;
    println!("Started Socket Listener at '{}'", socket_name);

    // !SECTION

    println!("Wormhole running!");

    loop {
        if tokio::select! {
            Ok(stream) = socket_listener.accept() => handle_connection(pods, stream).await,
            Some((command, reply_tx)) = rx.recv() => handle_tcp_connection(command, reply_tx, pods).await,
            _ = signals_rx.recv() => true,
        } {
            rest_service.abort();
            return Ok(());
        };
    }
}
