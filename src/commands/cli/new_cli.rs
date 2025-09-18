use std::collections::HashMap;

use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{
    tungstenite::{protocol::WebSocketConfig, Bytes},
    WebSocketStream,
};

use crate::{
    error::WhError,
    network::{ip::IpP, peer_ipc::PeerIPC, server::Server},
    pods::whpath::WhPath,
};

const DEFAULT_CLI_ADDRESS: &str = "0.0.0.0:8080";
const MAX_TRY_PORTS: u16 = 15;
const MAX_PORT: u16 = 65535;

custom_error::custom_error! {CliRunnerError
    WhError{source: WhError} = "{source}",
    ProvidedIpNotAvailable {ip: IpP, err: std::io::Error} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMaxPort = "Unable to start cli_listener (excedeed max port)",
    AboveMaxTry = "Unable to start cli_listener (exedeed the number of tries)",
    InvalidRequest = "Invalid/Inexistant request from the client cli",
    InvalidRequestData = "Unable to understand the client cli message (check your client version)",
    InternalMessageError{pair_name: String} = "Internal tx/rx pair broken: {pair_name}",
}

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
pub struct PodCreationArgs {
    /// Name of the pod
    pub name: String,
    /// mount point to create the pod in. By default creates a pod from the folder in the working directory with the name of the pod
    #[arg(long = "mount", short = 'm')]
    pub mountpoint: Option<WhPath>,
    /// Local port for the pod to use
    #[arg(long, short = 'p', default_value = "40000")]
    pub port: String,
    /// Network to join
    #[arg(long, short)]
    pub url: Option<String>,
    /// Name for this pod to use as a machine name with the network. Defaults to your Machine's name
    #[arg(long, short = 'H')]
    pub hostname: Option<String>,
    /// url this Pod reports to other to reach it
    #[arg(long, short)]
    pub listen_url: Option<String>,
    /// Additional hosts to try to join from as a backup
    #[arg(raw = true)]
    pub additional_hosts: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum RemoveMode {
    /// Simply remove the pod from the network without losing any data from the network
    /// and leaving behind any data that was stored on the pod
    Simple,
    /// Remove the pod from the network without losing any data on the network,
    /// and clone all data from the network into the folder where the pod was
    /// making this folder into a real folder
    Clone,
    /// Remove the pod from the network and delete any data that was stored in the pod
    Clean,
    /// Remove this pod from the network without distributing its data to other nodes
    Take,
}

type PodInfo = (Option<String>, Option<WhPath>);
#[derive(Debug, Serialize, Deserialize)] // requires `derive` feature
pub enum CliRequest {
    /// Start the pod (name, path)
    StartPod(PodInfo),
    /// Stop the pod (name, path)
    StopPod(PodInfo),
    /// Create a new pod and join a network if he have peers in arguments or create a new network
    New(PodCreationArgs),
    /// Inspect a pod with its configuration, connections, etc
    Inspect(PodInfo),
    /// Get hosts for a specific file
    GetHosts(PodInfo),
    /// Tree the folder structure from the given path and show hosts for each file
    Tree(PodInfo),
    /// Checks that the service is working (should print it's ip)
    ServiceStatus,
    /// Remove a pod from its network
    Remove(PodInfo, RemoveMode),
    /// Apply a new configuration to a pod
    Apply(PodInfo, Vec<String>),
    /// Restore many or a specific file configuration
    Restore(PodInfo, Vec<String>),
    /// Stops the service
    Interrupt,
}

#[derive(Debug, Serialize, Deserialize)] // requires `derive` feature
pub enum CliAnswer {}

struct CliState {
    listener: TcpListener,
    connected_endpoints: HashMap<usize, PeerIPC>,
}

// NOTE - later could keep the receiver to keep longer connection with the cli client
struct CliEndpoint {
    writer:
        SplitSink<WebSocketStream<tokio::net::TcpStream>, tokio_tungstenite::tungstenite::Message>,
}

impl CliEndpoint {
    async fn send(&mut self, message: CliAnswer) -> Result<(), CliRunnerError> {
        self.writer
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                bincode::serialize(&message).unwrap().into(),
            ))
            .await
            .map_err(|_| WhError::NetworkDied {
                called_from: "CliEndpoint::send".to_owned(),
            })?;
        Ok(())
    }
}

/// Create the tcp listener for the cli
///
/// If `ip` is provided, will only try using it.
/// If `ip` is not provided, will use `DEFAULT_CLI_ADDRESS` and increment the port up to `MAX_TRY_PORTS` until success.
async fn create_listener(ip: Option<IpP>) -> Result<(TcpListener, IpP), CliRunnerError> {
    if let Some(ip) = ip {
        Ok((
            TcpListener::bind(&ip.to_string()).await.map_err(|e| {
                CliRunnerError::ProvidedIpNotAvailable {
                    ip: ip.clone(),
                    err: e,
                }
            })?,
            ip,
        ))
    } else {
        let mut ip: IpP = IpP::try_from(DEFAULT_CLI_ADDRESS).unwrap();
        let mut current_try = 0;
        let mut listener = TcpListener::bind(&ip.to_string()).await;

        while let Err(e) = listener {
            log::warn!("Cli listener can't use address {ip} because of {e}");
            ip.set_port(ip.port + 1);
            current_try += 1;

            if current_try > MAX_TRY_PORTS {
                return Err(CliRunnerError::AboveMaxTry);
            };
            if ip.port > MAX_PORT {
                return Err(CliRunnerError::AboveMaxPort);
            }
            listener = TcpListener::bind(&ip.to_string()).await;
        }
        Ok((listener.unwrap(), ip))
    }
}

async fn cli_accept_job(
    listener: TcpListener,
    mut stop_rx: UnboundedReceiver<()>,
    ask_job: UnboundedSender<(CliRequest, CliEndpoint)>,
) -> Result<(), CliRunnerError> {
    while let Some(Ok((stream, _))) = tokio::select! {
        v = listener.accept() => Some(v),
        _ = stop_rx.recv() => None,
    } {
        let (writer, mut reader) = match tokio_tungstenite::accept_async_with_config(
            stream,
            Some(
                WebSocketConfig::default()
                    .max_message_size(None)
                    .max_frame_size(None),
            ),
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                log::error!("cli_accept_job: can't accept tcp stream: {}", e);
                continue;
            }
        }
        .split();

        let message_data = match reader.next().await {
            Some(Ok(msg)) => msg.into_data(),
            _ => {
                log::error!("{}", CliRunnerError::InvalidRequest);
                continue;
            }
        };

        let request: CliRequest = match bincode::deserialize(&message_data) {
            Ok(data) => data,
            Err(_) => {
                log::error!("{}", CliRunnerError::InvalidRequestData);
                continue;
            }
        };

        if let Err(_) = ask_job.send((request, CliEndpoint { writer })) {
            return Err(CliRunnerError::InternalMessageError {
                pair_name: "ask_job from cli_accept_job".to_owned(),
            });
        }
    }
    Ok(())
}

async fn cli(ip: Option<IpP>) -> Result<(), CliRunnerError> {
    let (listener, ip) = create_listener(ip).await?;
    Ok(())
}
