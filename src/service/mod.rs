pub mod clap;

use crate::ipc::error::ListenerError;
use crate::ipc::service::socket::new_socket_listener;
use crate::ipc::service::tcp::new_tcp_listener;
use crate::pods::pod::Pod;
use crate::pods::save::{delete_saved_pods, load_saved_pods};
use crate::service::clap::ServiceArgs;
use interprocess::local_socket::tokio::Listener;
use interprocess::local_socket::traits::tokio::Listener as TokioListenerExt;
use std::collections::HashMap;
use std::process::ExitCode;
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct Service {
    pub pods: HashMap<String, Pod>,
    pub socket: String,
    tcp_listener: TcpListener,
    socket_listener: Listener,
}

impl Service {
    pub async fn new(args: ServiceArgs) -> Option<Self> {
        let tcp_listener = new_tcp_listener(args.ip)
            .await
            .inspect_err(|err| eprintln!("{err}"))
            .ok()?;
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
            socket,
            tcp_listener,
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
        // SECTION WEB API

        let (tx, mut rx) = mpsc::channel::<(Command, oneshot::Sender<String>)>(100);
        let rest_app = axum::Router::new()
            .route("/", axum::routing::get(rest_app_handler))
            .with_state(tx);
        let rest_service = tokio::spawn(axum::serve(self.tcp_listener, rest_app).into_future());

        // !SECTION

        // SECTION OS-PIPES

        // !SECTION

        println!("Wormhole running!");

        loop {
            if tokio::select! {
                Ok(stream) = self.socket_listener.accept() => self.handle_connection(stream).await,
                Some((command, reply_tx)) = rx.recv() => handle_tcp_connection(command, reply_tx, pods).await,
                _ = signals_rx.recv() => true,
            } {
                rest_service.abort();
                return Ok(());
            };
        }
    }
}
