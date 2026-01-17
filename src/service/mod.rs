pub mod clap;
pub mod commands;
pub mod connection;
pub mod socket;
pub mod tcp;

use crate::ipc::commands::Command;
use crate::ipc::error::ListenerError;
use crate::pods::pod::{Pod, PodPrototype};
use crate::pods::save::{delete_saved_pods, load_saved_pods};
use crate::service::clap::ServiceArgs;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::traits::tokio::Listener as TokioListenerExt;
use socket::new_socket_listener;
use std::collections::HashMap;
use std::future::IntoFuture;
use std::process::ExitCode;
use tcp::new_tcp_listener;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::oneshot::{self, Sender};

pub struct Service {
    pub pods: HashMap<String, Pod>,
    pub frozen_pods: HashMap<String, PodPrototype>,
    pub socket: String,
    rest_service: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    web_request_rx: Receiver<(Command, Sender<String>)>,
    socket_listener: Listener,
}

impl Service {
    pub async fn new(args: ServiceArgs) -> Option<Self> {
        let tcp_listener = new_tcp_listener(args.ip)
            .await
            .inspect_err(|err| eprintln!("{err}"))
            .ok()?;
        let (tx, rx) = mpsc::channel::<(Command, oneshot::Sender<String>)>(100);
        let rest_app: axum::Router = axum::Router::new()
            .route("/", axum::routing::get(rest_app_handler))
            .with_state(tx);
        let rest_service: tokio::task::JoinHandle<Result<(), std::io::Error>> =
            tokio::spawn(axum::serve(tcp_listener, rest_app).into_future());

        let (socket_listener, socket) = new_socket_listener(args.socket)
            .inspect_err(|err| eprintln!("{err}"))
            .ok()?;
        let mut pods = HashMap::new();

        if args.clean {
            delete_saved_pods(&socket)
                .inspect_err(|err| eprintln!("Failed to delete saved pods: {:?}", err))
                .ok()?;
        } else {
            load_saved_pods(&mut pods, &socket)
                .await
                .inspect_err(|err| eprintln!("Failed to load saved pods: {:?}", err))
                .ok()?;
        }

        Some(Service {
            pods,
            frozen_pods: HashMap::new(),
            socket,
            rest_service,
            web_request_rx: rx,
            socket_listener,
        })
    }

    pub async fn stop_all_pods(self) -> ExitCode {
        let mut status = ExitCode::SUCCESS;
        for (name, pod) in self.pods.into_iter() {
            if pod.should_restart {
                let _ = pod
                    .save(&self.socket)
                    .await
                    .inspect_err(|err| log::error!("Couldn't save the pod data: {err}"));
            }

            match pod.stop().await {
                Ok(()) => log::info!("Stopped pod '{name}'"),
                Err(e) => {
                    eprintln!("Pod '{name}' failed be stopped: {e}");
                    status = ExitCode::FAILURE
                }
            }
        }
        log::info!("Wormhole stopped");
        status
    }

    pub async fn start_commands_listeners(
        &mut self,
        mut signals_rx: UnboundedReceiver<()>,
    ) -> Result<(), ListenerError> {
        println!("Wormhole running!");

        loop {
            if tokio::select! {
                Ok(stream) = self.socket_listener.accept() => self.handle_connection(stream).await,
                Some((command, reply_tx)) = self.web_request_rx.recv() => self.handle_tcp_connection(command, reply_tx).await,
                _ = signals_rx.recv() => true,
            } {
                self.rest_service.abort();
                return Ok(());
            };
        }
    }
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
