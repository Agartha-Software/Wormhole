use std::{collections::HashMap, future::Future, sync::Arc};

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_tungstenite::{
    tungstenite::{protocol::WebSocketConfig, Bytes},
    WebSocketStream,
};

use crate::{
    commands::cli::new_cli::common::{CliAnswer, CliMode, CliRequest},
    error::WhError,
    network::{ip::IpP, peer_ipc::PeerIPC, server::Server},
    pods::{arbo::LOCK_TIMEOUT, whpath::WhPath},
};

const DEFAULT_CLI_ADDRESS: &str = "0.0.0.0:8080";
const MAX_TRY_PORTS: u16 = 15;
const MAX_PORT: u16 = 65535;

custom_error::custom_error! {CliRunnerError
    WhError{source: WhError} = "{source}",
    ProvidedIpNotAvailable {ip: IpP, err: std::io::Error} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMaxPort = "Unable to start cli_listener (excedeed max port)",
    AboveMaxTry = "Unable to start cli_listener (exedeed the number of tries)",
    InvalidRequest {req_id: usize} = "Invalid request from the client cli (check your client version)",
    InvalidRegister = "Unable to understand the first client cli message (check your client version)",
    InternalMessageError{pair_name: String} = "Internal tx/rx pair broken: {pair_name}",
}

// NOTE - later could keep the receiver to keep longer connection with the cli client
struct CliEndpoint {
    pub mode: CliMode,
    writer: SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
    reader_fwd: Option<JoinHandle<()>>,
}

impl CliEndpoint {
    async fn new(
        writer: SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
        mut reader: SplitStream<WebSocketStream<TcpStream>>,
        req_arrival_tx: UnboundedSender<InternalRequest>,
        id: usize,
    ) -> Result<Self, CliRunnerError> {
        let mut proto = CliEndpoint {
            mode: CliMode::Oneshot,
            writer,
            reader_fwd: None,
        };

        if let Ok(Some(CliRequest::Register(mode))) = Self::read(&mut reader).await {
            proto.mode = mode;
            proto.reader_fwd = Some(tokio::spawn(Self::forwarder(reader, req_arrival_tx, id)));
            Ok(proto)
        } else {
            Err(CliRunnerError::InvalidRegister)
        }
    }

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

    async fn read(
        reader: &mut SplitStream<WebSocketStream<TcpStream>>,
    ) -> Result<Option<CliRequest>, CliRunnerError> {
        let message_data = match reader.next().await {
            Some(Ok(msg)) => msg.into_data(),
            _ => return Ok(None),
        };

        bincode::deserialize(&message_data)
            .map_err(|_| CliRunnerError::InvalidRequest { req_id: 0 })
    }

    async fn forwarder(
        mut reader: SplitStream<WebSocketStream<TcpStream>>,
        tx: UnboundedSender<InternalRequest>,
        id: usize,
    ) {
        loop {
            let message_data = match reader.next().await {
                Some(Ok(msg)) => msg.into_data(),
                None => continue,
                Some(Err(e)) => {
                    log::error!("cli forwarder unexpected close: {e}");
                    return;
                }
            };

            let request: Result<CliRequest, CliRunnerError> = bincode::deserialize(&message_data)
                .map_err(|_| CliRunnerError::InvalidRequest { req_id: id });
            tx.send((id, request)).expect("cli forwarder tx closed");
        }
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

/// Accepts new connection request from any cli client
/// Add them in the provided `endpoints`
async fn cli_accept_watchdog(
    listener: TcpListener,
    endpoints: Arc<RwLock<HashMap<usize, CliEndpoint>>>,
    req_arrival_tx: UnboundedSender<InternalRequest>,
) -> Result<(), CliRunnerError> {
    while let Ok((stream, _)) = listener.accept().await {
        let (writer, reader) = match tokio_tungstenite::accept_async_with_config(
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
                log::error!("cli_accept_watchdog: can't accept tcp stream: {}", e);
                continue;
            }
        }
        .split();

        if let Some(mut endpoints) = endpoints.try_write_for(LOCK_TIMEOUT) {
            let key = next_available_key(&endpoints);
            let new_endpoint =
                CliEndpoint::new(writer, reader, req_arrival_tx.clone(), key).await?;
            endpoints.insert(key, new_endpoint);
        } else {
            log::error!("cli_accept_watchdog: can't lock endpoints mutex");
        }
    }
    Ok(())
}

async fn cli(ip: Option<IpP>) -> Result<(), CliRunnerError> {
    let (listener, ip) = create_listener(ip).await?;
    let endpoints: Arc<RwLock<HashMap<usize, CliEndpoint>>> = Default::default();
    let (req_arrival_tx, req_arrival_rx) = mpsc::unbounded_channel::<InternalRequest>();

    let accept_watchdog = cli_accept_watchdog(listener, endpoints, req_arrival_tx);
    Ok(())
}

fn next_available_key<T>(hashmap: &HashMap<usize, T>) -> usize {
    let mut key: usize = 0;
    while hashmap.contains_key(&key) {
        key += 1;
    }
    key
}

type InternalRequest = (usize, Result<CliRequest, CliRunnerError>);
