use std::{collections::HashMap, future::Future, sync::Arc};

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
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
    InvalidRequest = "Invalid request from the client cli (check your client version)",
    InvalidRegister = "Unable to understand the first client cli message (check your client version)",
    InternalMessageError{pair_name: String} = "Internal tx/rx pair broken: {pair_name}",
}

// NOTE - later could keep the receiver to keep longer connection with the cli client
struct CliEndpoint {
    pub mode: CliMode,
    writer: SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
    reader: SplitStream<WebSocketStream<TcpStream>>,
}

impl CliEndpoint {
    async fn new(
        writer: SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
        reader: SplitStream<WebSocketStream<TcpStream>>,
    ) -> Result<Self, CliRunnerError> {
        let mut proto = CliEndpoint {
            writer,
            reader,
            mode: CliMode::Oneshot,
        };

        if let Ok(Some(CliRequest::Register(mode))) = proto.read().await {
            proto.mode = mode;
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

    async fn read(&mut self) -> Result<Option<CliRequest>, CliRunnerError> {
        let message_data = match self.reader.next().await {
            Some(Ok(msg)) => msg.into_data(),
            _ => return Ok(None),
        };

        bincode::deserialize(&message_data).map_err(|_| CliRunnerError::InvalidRequest)
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

        let new_endpoint = CliEndpoint::new(writer, reader).await?;
        if let Some(mut endpoints) = endpoints.try_write_for(LOCK_TIMEOUT) {
            let key = next_available_key(&endpoints);
            endpoints.insert(key, new_endpoint);
        } else {
            log::error!("cli_accept_watchdog: can't lock endpoints mutex");
        }
    }
    Ok(())
}

async fn listen_watchdog(endpoints: Arc<RwLock<HashMap<usize, CliEndpoint>>>) {
    loop {
        if let Some(mut endpoints) = endpoints.try_write_for(LOCK_TIMEOUT) {
            let mut requests: Vec<(usize, CliRequest)> = Vec::new();

            for (key, endp) in endpoints.iter_mut() {
                match endp.read().await {
                    Err(e) => {
                        let _ = endp.send(CliAnswer::Error(e.to_string())).await;
                        endp.mode = CliMode::ToDelete;
                        ()
                    }
                    Ok(None) => (),
                    Ok(Some(req)) => requests.push((*key, req)),
                }
            }
        } else {
            log::error!("listen_watchdog: can't lock endpoints mutex");
            continue;
        };
    }
}

async fn cli(ip: Option<IpP>) -> Result<(), CliRunnerError> {
    let (listener, ip) = create_listener(ip).await?;
    let endpoints: Arc<RwLock<HashMap<usize, CliEndpoint>>> = Default::default();

    let accept_watchdog = cli_accept_watchdog(listener, endpoints);
    Ok(())
}

fn next_available_key<T>(hashmap: &HashMap<usize, T>) -> usize {
    let mut key: usize = 0;
    while hashmap.contains_key(&key) {
        key += 1;
    }
    key
}
