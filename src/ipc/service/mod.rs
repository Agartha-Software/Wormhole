mod commands;
mod connection;
mod socket;
mod tcp;

use crate::{
    ipc::{
        commands::Command,
        error::{ListenerError, TCPListenerError},
        service::{
            connection::{handle_command, handle_connection},
            socket::new_socket_listener,
            tcp::new_free_tcp_listener,
        },
    },
    pods::pod::Pod,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use either::Either;
use interprocess::local_socket::{self, traits::tokio::Listener};
use std::{collections::HashMap, future::IntoFuture};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{self, UnboundedReceiver},
        oneshot,
    },
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

    let (tx, mut rx) = mpsc::channel::<(Command, oneshot::Sender<String>)>(100);

    let rest_app = axum::Router::new()
        .route("/", axum::routing::get(rest_app_handler))
        .with_state(tx);
    let rest_service = tokio::spawn(axum::serve(tcp_listener, rest_app).into_future());

    let socket_name = specific_socket.unwrap_or(SOCKET_DEFAULT_NAME.to_string());
    let socket_listener = new_socket_listener(&socket_name)?;
    println!("Started Socket Listener at '{}'", socket_name);
    println!("Wormhole running!");

    loop {
        if tokio::select! {
            Ok(stream) = socket_listener.accept() => handle_connection(pods, stream).await,
            Some((command, reply_tx)) = rx.recv() => {
                let mut buffer = String::new();
                let mut stream = Either::Right(&mut buffer);
                let _ = handle_command::<local_socket::tokio::Stream>(command, pods, &mut stream).await;
                let _ = reply_tx.send(buffer);
                false
            },
            _ = signals_rx.recv() => true,
        } {
            rest_service.abort();
            return Ok(());
        };
    }
}

async fn rest_app_handler(
    State(tx): State<mpsc::Sender<(Command, oneshot::Sender<String>)>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let command: Result<Command, serde_json::Error> = serde_json::from_value(payload);

    let command = match command {
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        Ok(c) => c,
    };

    let (reply_tx, reply_rx) = oneshot::channel();
    if tx.send((command, reply_tx)).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    match reply_rx.await {
        Ok(response) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            response,
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
