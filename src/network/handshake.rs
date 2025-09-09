use custom_error::custom_error;
use futures::{future::Either, Sink};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_tungstenite::{
    tungstenite::{self, Message},
    MaybeTlsStream, WebSocketStream,
};

use crate::{
    config::{GlobalConfig, LocalConfig},
    error::WhError,
    pods::{arbo::Arbo, network::network_interface::NetworkInterface},
};

custom_error! {
    pub HandshakeError
    CouldntConnect = "peer did not respond",
    InvalidHandshake = "peer behaved unexpectedly",
    Tungstenite{source: tungstenite::Error} = "tungstenite: {source}",
    Serialization{bincode: bincode::Error} = "bincode: {bincode}",
    WhError{source: WhError} = "{source}",
    Remote{serialized: RemoteHandshakeError} = "{serialized}",
}

custom_error! {
    /// WARNING: make sure this struct is kept in sync with HandshakeError
    #[derive(Serialize, Deserialize, Clone)]
    pub RemoteHandshakeError
    CouldntConnect = "peer did not respond",
    InvalidHandshake = "peer behaved unexpectedly",
    Tungstenite{string: String} = "tungstenite: {string}",
    Serialization{string: String} = "bincode: {string}",
    WhError{string: String} = "{string}",
}

impl From<&HandshakeError> for RemoteHandshakeError {
    fn from(value: &HandshakeError) -> Self {
        type E = HandshakeError;
        type ES = RemoteHandshakeError;
        match value {
            E::CouldntConnect => ES::CouldntConnect,
            E::InvalidHandshake => ES::InvalidHandshake,
            E::Tungstenite { source } => ES::Tungstenite {
                string: source.to_string(),
            },
            E::Serialization { bincode } => ES::Serialization {
                string: bincode.to_string(),
            },
            E::WhError { source } => ES::WhError {
                string: source.to_string(),
            },
            E::Remote { serialized } => serialized.clone(),
        }
    }
}

impl From<RemoteHandshakeError> for HandshakeError {
    fn from(value: RemoteHandshakeError) -> Self {
        type E = HandshakeError;
        type ES = RemoteHandshakeError;
        match value {
            ES::CouldntConnect => E::CouldntConnect,
            ES::InvalidHandshake => E::InvalidHandshake,
            other => E::Remote {
                serialized: other.clone(),
            },
        }
    }
}

impl From<bincode::Error> for HandshakeError {
    fn from(bincode: bincode::Error) -> Self {
        HandshakeError::Serialization { bincode }
    }
}

const GIT_HASH: &'static str = env!("GIT_HASH");

#[derive(Deserialize, Serialize)]
pub enum Handshake {
    /// First Message.
    /// Sent by connecting peers for entry into a network
    /// Valid replies: [Handshake::Accept], [Handshake::Accept]
    Connect(Connect),

    /// Second Message.
    /// Returned by entrypoint to entrant to complete the handshake
    /// Contains important information about this network
    /// Valid replies: Move on to [crate::network::message::MessageContent]
    Accept(Accept),

    /// Alternative Second Message.
    /// Returned by entrypoint to entrant to interrupt the handshake
    /// Valid replies: Shut down. Retry ?
    Refuse(RemoteHandshakeError),

    /// Simple Acknowledge Message
    /// Symmetric first and second message
    /// Send first by connecting peers and returned by accepting peers
    /// Valid replies: [Handshake::Wave] then [crate::network::message::MessageContent]
    Wave(Wave),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Accept {
    /// hostname of the accepting peer
    pub hostname: String,

    /// rename the connecting host, in case of name collisions
    pub rename: Option<String>,

    /// hostname of peers, in arbitrary order, with the accepting host first
    pub hosts: Vec<String>,

    /// urls of all avaialble peers, in the same order
    pub urls: Vec<Option<String>>,

    /// global config of the network
    pub config: GlobalConfig,

    /// ITree of the network
    pub arbo: Arbo,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Connect {
    /// version string to detect incompatible serialization
    pub magic_version: String,

    /// hostname of the connecting peer
    pub hostname: String,

    /// url by which this peer may be accessed, if available
    pub url: Option<String>,
    // /// simple unencrypted passkey
    // pub authentification: String,

    // /// startup contributions
    // pub files: HashMap<WhPath, Metadata>
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Wave {
    /// hostname of the waving peer
    pub hostname: String,

    /// url by which the waving peer may be reached, if available
    pub url: Option<String>,

    /// hostname of the third party peer that acted as an entrypoint
    pub blame: String,
}

fn unique_hostname(mut hostname: String, colliders: &Vec<String>) -> Option<String> {
    if !colliders.contains(&hostname) {
        return None;
    }
    while colliders.contains(&hostname) {
        println!("failed: {hostname}");
        if let Some((prefix, realname)) = hostname.split_once('.') {
            if let Ok(idx) = prefix.parse::<usize>() {
                hostname = format!("{}.{}", idx + 1, realname);
            } else if &prefix == &"" {
                hostname = format!("1{}", hostname);
            } else {
                hostname = format!("1.{}", hostname);
            }
        } else {
            hostname = format!("1.{}", hostname);
        }
    }
    Some(hostname)
}

pub async fn accept(
    stream: &mut SplitStream<WebSocketStream<TcpStream>>,
    sink: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    network: &NetworkInterface,
) -> Result<Either<Connect, Wave>, HandshakeError> {
    let message = stream
        .next()
        .await
        .map(|res| res.map_err(Into::into))
        .unwrap_or(Err(HandshakeError::InvalidHandshake));
    let handshake = match message {
        Ok(Message::Binary(bytes)) => bincode::deserialize::<Handshake>(&bytes).map_err(From::from),
        Ok(_) => Err(HandshakeError::InvalidHandshake),
        Err(e) => Err(e.into()),
    };

    let result = match handshake {
        Ok(Handshake::Connect(mut connect)) => {
            (async || {
                // closures to capture ? process
                let hostname = network.hostname()?;
                let url = network.url.clone();
                let url_pairs = network
                    .peers
                    .read()
                    .iter()
                    .map(|peer| (peer.hostname.clone(), peer.url.clone()))
                    .collect::<Vec<_>>();

                let (hosts, urls) = [(hostname.clone(), url)]
                    .into_iter()
                    .chain(url_pairs.into_iter())
                    .inspect(|(h, u)| log::trace!("accept:h{h}, u{u:?}"))
                    .unzip();
                let rename = unique_hostname(connect.hostname.clone(), &hosts);

                if let Some(rename) = &rename {
                    connect.hostname = rename.clone();
                }

                let accept = Accept {
                    urls,
                    hosts,
                    rename,
                    hostname,
                    config: network.global_config.read().clone(),
                    arbo: (*network.arbo.read()).clone(),
                };
                let data = bincode::serialize(&Handshake::Accept(accept))?;
                sink.send(Message::Binary(data.into())).await?;

                Ok(Either::Left(connect))
            })()
            .await
        }
        Ok(Handshake::Wave(wave)) => {
            (async || {
                // closures to capture ? process
                let wave_back = Wave {
                    hostname: network.hostname()?,
                    url: None,
                    blame: wave.hostname.clone(),
                };
                let data = bincode::serialize(&Handshake::Wave(wave_back))?;
                sink.send(Message::Binary(data.into())).await?;

                Ok(Either::Right(wave))
            })()
            .await
        }
        Ok(_) => Err(HandshakeError::InvalidHandshake),
        Err(e) => Err(e),
    };
    let result = if let Err(error) = result {
        (async || {
            sink.send(Message::Binary(
                bincode::serialize(&Handshake::Refuse((&error).into()))?.into(),
            ))
            .await?;
            Err(error)
        })()
        .await
    } else {
        result
    };
    result
}

pub async fn wave<T>(
    stream: &mut SplitStream<WebSocketStream<T>>,
    sink: &mut SplitSink<WebSocketStream<T>, Message>,
    hostname: String,
    blame: String,
) -> Result<Wave, HandshakeError>
where
    SplitStream<WebSocketStream<T>>: StreamExt<Item = Result<Message, tungstenite::Error>>,
    WebSocketStream<T>: Sink<Message, Error = tungstenite::Error>,
{
    let wave = Wave {
        hostname,
        url: None,
        blame,
    };

    let serialized = bincode::serialize(&Handshake::Wave(wave))?;
    sink.send(Message::Binary(serialized.into())).await?;

    let response = stream
        .next()
        .await
        .ok_or(HandshakeError::InvalidHandshake)??;

    let handshake = if let Message::Binary(bytes) = response {
        Ok(bincode::deserialize::<Handshake>(&bytes)?)
    } else {
        Err(HandshakeError::InvalidHandshake)
    }?;

    match handshake {
        Handshake::Wave(wave) => Ok(wave),
        _ => Err(HandshakeError::InvalidHandshake),
    }
}

pub async fn connect(
    stream: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    local_config: &LocalConfig,
) -> Result<Accept, HandshakeError> {
    let connect = Connect {
        hostname: local_config.general.hostname.clone(),
        magic_version: GIT_HASH.into(),
        url: local_config.general.url.clone(),
    };

    let serialized = bincode::serialize(&Handshake::Connect(connect))?;
    sink.send(Message::Binary(serialized.into())).await?;

    let response = stream
        .next()
        .await
        .ok_or(HandshakeError::InvalidHandshake)??;

    let handshake = if let Message::Binary(bytes) = response {
        Ok(bincode::deserialize::<Handshake>(&bytes)?)
    } else {
        Err(HandshakeError::InvalidHandshake)
    }?;

    match handshake {
        Handshake::Accept(accept) => Ok(accept),
        Handshake::Refuse(error) => Err(error.into()),
        _ => Err(HandshakeError::InvalidHandshake),
    }
}
