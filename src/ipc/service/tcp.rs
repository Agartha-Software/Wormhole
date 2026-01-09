use crate::ipc::service::handle_command;
use crate::pods::pod::Pod;
use crate::{
    ipc::{commands::Command, error::TCPListenerError},
    network::ip::IpP,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use either::Either;
use interprocess::local_socket;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::{net::TcpListener, sync::oneshot};

const MAX_TRY_PORTS: u16 = 10;
const DEFAULT_ADDRESS: &str = "127.0.0.1:8081";

pub async fn new_tcp_listener(
    specific_ip: Option<String>,
) -> Result<TcpListener, TCPListenerError> {
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
        None => free_tcp_listener().await?,
    };
    println!("Started Tcp Listener at '{ip}'");
    Ok(tcp_listener)
}

async fn free_tcp_listener() -> Result<(TcpListener, String), TCPListenerError> {
    let mut ip: IpP = IpP::try_from(DEFAULT_ADDRESS).expect("Invalid ip provided");

    let mut port_tries_count = 0;
    loop {
        match TcpListener::bind(&ip.to_string()).await {
            Ok(listener) => break Ok((listener, ip.to_string())),
            Err(err) => {
                if ip.port == u16::MAX {
                    break Err(TCPListenerError::AboveMainPort { max_port: u16::MAX });
                }
                if port_tries_count > MAX_TRY_PORTS {
                    break Err(TCPListenerError::AboveMaxTry {
                        max_try_port: MAX_TRY_PORTS,
                    });
                }
                log::warn!("Address {ip} not available due to {err}, switching...",);
                ip.set_port(ip.port + 1);
                port_tries_count += 1;
            }
        }
    }
}

/// Borrow pods, execute the command, sends the answer to the web api
pub async fn handle_tcp_connection(
    command: Command,
    reply_tx: oneshot::Sender<String>,
    pods: &mut HashMap<String, Pod>,
) -> bool {
    let mut buffer = String::new();
    let mut stream = Either::Right(&mut buffer);
    let _ = handle_command::<local_socket::tokio::Stream>(command, pods, &mut stream).await;
    let _ = reply_tx.send(buffer);
    false
}

// As the rest_app server is a forever going task, it can't borrow pods
// Instead it uses a tx to trigger the same select! as the os-pipe handler
pub async fn rest_app_handler(
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
