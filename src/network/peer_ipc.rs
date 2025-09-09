use futures::future::Either;
use futures_util::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, UnboundedSender},
};
use tokio_tungstenite::tungstenite::{protocol::WebSocketConfig, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::{
    config::LocalConfig,
    network::{
        forward::{forward_peer_to_receiver, forward_sender_to_peer},
        handshake::{self, Accept, HandshakeError, Wave},
    },
    pods::network::network_interface::NetworkInterface,
};

use super::message::{Address, FromNetworkMessage, MessageAndStatus};

#[derive(Debug)]
pub struct PeerIPC {
    pub url: Option<String>,
    pub hostname: String,
    pub thread: tokio::task::JoinHandle<()>,
    pub sender: mpsc::UnboundedSender<MessageAndStatus>,
    // pub receiver: mpsc::Receiver<NetworkMessage>, // receive a message from the peer
}

impl PeerIPC {
    async fn work(
        peer_write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        peer_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        receiver_in: mpsc::UnboundedSender<FromNetworkMessage>,
        mut sender_out: mpsc::UnboundedReceiver<MessageAndStatus>,
        peer: String,
    ) {
        tokio::join!(
            forward_peer_to_receiver(peer_read, receiver_in, peer.clone()),
            forward_sender_to_peer(peer_write, &mut sender_out, peer)
        );
    }

    async fn work_from_incomming(
        peer_write: SplitSink<WebSocketStream<TcpStream>, Message>,
        peer_read: SplitStream<WebSocketStream<TcpStream>>,
        receiver_in: mpsc::UnboundedSender<FromNetworkMessage>,
        mut sender_out: mpsc::UnboundedReceiver<MessageAndStatus>,
        peer: Address,
    ) {
        tokio::join!(
            forward_peer_to_receiver(peer_read, receiver_in, peer.clone()),
            forward_sender_to_peer(peer_write, &mut sender_out, peer)
        );
    }

    pub async fn accept(
        network_interface: &NetworkInterface,
        stream: WebSocketStream<TcpStream>,
        receiver_in: UnboundedSender<FromNetworkMessage>,
    ) -> Result<Self, HandshakeError> {
        let (sender_in, sender_out) = mpsc::unbounded_channel();

        let (mut sink, mut stream) = stream.split();

        let (hostname, url) =
            match handshake::accept(&mut stream, &mut sink, network_interface).await? {
                Either::Left(connect) => (connect.hostname, connect.url),
                Either::Right(wave) => (wave.hostname, wave.url),
            };

        Ok(Self {
            thread: tokio::spawn(Self::work_from_incomming(
                sink,
                stream,
                receiver_in,
                sender_out,
                hostname.clone(),
            )),
            url,
            sender: sender_in,
            hostname,
        })
    }

    pub async fn connect(
        url: String,
        config: &LocalConfig,
        receiver_in: UnboundedSender<FromNetworkMessage>,
    ) -> Result<(Self, Accept), HandshakeError> {
        let (sender_in, sender_out) = mpsc::unbounded_channel();

        log::trace!("connecting to ws://{url}");
        let (accept, thread) = match tokio_tungstenite::connect_async_with_config(
            "ws://".to_string() + &url,
            Some(
                WebSocketConfig::default()
                    .max_message_size(None)
                    .max_frame_size(None),
            ),
            false,
        )
        .await
        {
            Ok((stream, _)) => {
                let (mut sink, mut stream) = stream.split();
                let accept = handshake::connect(&mut stream, &mut sink, &config).await?;
                (
                    accept,
                    tokio::spawn(Self::work(
                        sink,
                        stream,
                        receiver_in,
                        sender_out,
                        url.clone(),
                    )),
                )
            }
            Err(e) => {
                log::warn!("failed to connect to {}. Error: {}", url, e);
                return Err(e.into());
            }
        };
        Ok((
            Self {
                thread,
                url: Some(url),
                hostname: accept.hostname.clone(),
                sender: sender_in,
            },
            accept,
        ))
    }

    pub async fn wave(
        url: String,
        hostname: String,
        blame: String,
        receiver_in: UnboundedSender<FromNetworkMessage>,
    ) -> Result<(PeerIPC, Wave), HandshakeError> {
        let (sender_in, sender_out) = mpsc::unbounded_channel();

        log::trace!("waving to ws://{url}");
        let (wave, thread) = match tokio_tungstenite::connect_async_with_config(
            "ws://".to_string() + &url,
            Some(
                WebSocketConfig::default()
                    .max_message_size(None)
                    .max_frame_size(None),
            ),
            false,
        )
        .await
        {
            Ok((stream, _)) => {
                let (mut sink, mut stream) = stream.split();
                let wave = handshake::wave(&mut stream, &mut sink, hostname, blame).await?;
                (
                    wave,
                    tokio::spawn(Self::work(
                        sink,
                        stream,
                        receiver_in,
                        sender_out,
                        url.clone(),
                    )),
                )
            }
            Err(e) => {
                log::warn!("failed to connect to {}. Error: {}", url, e);
                return Err(e.into());
            }
        };
        Ok((
            Self {
                thread,
                url: Some(url),
                hostname: wave.hostname.clone(),
                sender: sender_in,
            },
            wave,
        ))
    }

    // start connexions to peers
    pub async fn peer_startup<I: IntoIterator<Item = String>>(
        peer_entrypoints: I,
        hostname: String,
        blame: String,
        receiver_in: UnboundedSender<FromNetworkMessage>,
    ) -> Result<Vec<PeerIPC>, HandshakeError> {
        futures_util::future::join_all(
            peer_entrypoints.into_iter().map(|url| {
                PeerIPC::wave(url, hostname.clone(), blame.clone(), receiver_in.clone())
            }),
        )
        .await
        .into_iter()
        .fold(Ok(vec![]), |acc, b: Result<_, _>| {
            acc.and_then(|mut acc| {
                acc.push(b?.0);
                Ok(acc)
            })
        })
    }
}

impl Drop for PeerIPC {
    fn drop(&mut self) {
        log::debug!("Dropping PeerIPC {}", self.hostname);
        self.thread.abort();
    }
}
