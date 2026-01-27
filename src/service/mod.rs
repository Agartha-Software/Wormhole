pub mod clap;
pub mod commands;
pub mod connection;
pub mod save;
pub mod socket;
pub mod tcp;

use crate::ipc::commands::Command;
use crate::ipc::error::ListenerError;
use crate::pods::pod::Pod;
use crate::pods::prototype::PodPrototype;
use crate::service::clap::ServiceArgs;
use crate::service::save::{delete_saved_pods, save_prototype, ServiceKey};
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
    pub nickname: String,
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

        let nickname = args
            .nickname
            .unwrap_or_else(|| gethostname::gethostname().to_string_lossy().to_string());

        let mut service = Service {
            pods: HashMap::new(),
            frozen_pods: HashMap::new(),
            socket,
            nickname,
            rest_service,
            web_request_rx: rx,
            socket_listener,
        };

        if args.clean {
            let service_key = ServiceKey::from_path(&service.socket);
            delete_saved_pods(&service_key)
                .inspect_err(|err| eprintln!("Failed to delete saved pods: {:?}", err))
                .ok()?;
        } else {
            service
                .load_saved_pods()
                .await
                .inspect_err(|err| eprintln!("Failed to load saved pods: {:?}", err))
                .ok()?;
        }

        Some(service)
    }

    pub async fn stop_all_pods(self) -> ExitCode {
        let service_key = ServiceKey::from_path(&self.socket);
        let mut status = ExitCode::SUCCESS;
        for (name, pod) in self.pods.into_iter() {
            if pod.should_restart {
                match pod.try_generate_prototype() {
                    Some(prototype) => {
                        let _ = save_prototype(prototype, &service_key, false)
                            .inspect_err(|e| log::error!("Couldn't save the pod data: {e:?}"));
                    }
                    None => log::error!("Couldn't access pod {} while saving.", name),
                }
            }

            match pod.stop().await {
                Ok(()) => log::info!("Stopped pod '{name}'"),
                Err(e) => {
                    eprintln!("Pod '{name}' failed be stopped: {e}");
                    status = ExitCode::FAILURE
                }
            }
        }

        for prototype in self.frozen_pods.into_values() {
            let _ = save_prototype(prototype, &service_key, true)
                .inspect_err(|e| log::error!("Couldn't save the frozen pod data: {e:?}"));
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
