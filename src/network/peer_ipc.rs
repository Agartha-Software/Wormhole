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

/// Represents a running websocket peer connection.
#[derive(Debug)]
pub struct PeerIPC {
    /// Optional URL used to connect to the peer.
    /// None if the peer initiated the connection.
    pub url: Option<String>,
    /// hostname annouced by the remote peer during handshake.
    pub hostname: String,
    /// the `JoinHandle` for the worker task that forwards traffic between the local system and the peer.
    /// The task is aborted when the `PeerIPC` is dropped.
    pub thread: tokio::task::JoinHandle<()>,
    /// Channel for sending messages to the peer.
    /// Used by other parts of the system to send messages to this peer.
    pub sender: mpsc::UnboundedSender<MessageAndStatus>,
    // pub receiver: mpsc::Receiver<NetworkMessage>, // receive a message from the peer
}

impl PeerIPC {
    /// Worker routine for  an outbound or generic websocket stream.
    ///
    /// This function runs two tasks concurrently:
    /// 1. `forward_peer_to_receiver`: Reads messages from the peer and forwards them
    ///    to the local receiver channel.
    /// 2. `forward_sender_to_peer`: Reads messages from the local sender channel
    ///    and sends them to the peer.
    /// Both tasks run until the connection is closed or an error occurs.
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

    //FIXME - duplicate of work(), it's the same code
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

    /// Accept an incoming websocket connection and complete the handshake.
    ///
    /// This function performs the required handshake using
    /// `handshake::accept`, starts the worker task to forward messages,
    /// and returns an owned `PeerIPC` on success. If the handshake fails
    /// the underlying `HandshakeError` is returned.
    ///
    /// Parameters:
    /// - `network_interface`: reference to the local `NetworkInterface` used
    /// during the handshake.
    /// - `stream`: the websocket stream to accept.
    /// - `receiver_in`: channel used to forward incoming peer messages into
    /// the rest of the application.
    ///
    /// # Errors
    /// Returns `HandshakeError` when the handshake fails.
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

    /// Connect to a remote peer at the specified URL and complete the handshake.
    ///
    /// On success, returns an owned `PeerIPC` and the `Accept` information from
    /// the handshake. If the connection or handshake fails, returns a `HandshakeError`.
    /// The worker task is started to forward messages between the local system
    /// and the peer.
    ///
    /// Parameters:
    /// - `url`: the URL of the remote peer to connect to (e.g., "x.x.x.x:1234").
    /// - `config`: reference to the local `LocalConfig` used during the handshake.
    /// - `receiver_in`: channel that will receive messages coming from the peer
    ///
    /// # Errors
    /// Returns `HandshakeError` if the connection or handshake fails.
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

    /// Create a connnection to a peer and perform the "wave" handshake.
    ///
    /// This is similar to `connect`, but uses the "wave" handshake variant.
    /// A "wave" is a special outgoing handshacke variant implemented in the `handshake` module.
    /// On success this returns `(PeerIPC, Wave)` where
    /// `Wave` contains handshake-specific metadata.
    ///
    /// Parameters:
    /// - `url`: the URL of the remote peer to connect to (e.g., "x.x.x.x:1234").
    /// - `hostname`: the local hostname to announce during the handshake.
    /// - `blame`: the local blame string to announce during the handshake.
    /// - `receiver_in`: channel that will receive messages coming from the peer
    ///
    /// # Errors
    /// Returns `HandshakeError` if the connection or handshake fails.
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

    /// Start multiple outbound peer connections concurrently.
    ///
    /// `peer_entrypoints` is an iterator over `String` URLs (host:port). The
    /// function attempts to `wave` to each entrypoint concurrently using
    /// `futures_util::future::join_all`. The results are collected and
    /// returned as a `Vec<PeerIPC>` on success. If any handshake fails, the
    /// first encountered `HandshakeError` is returned.
    ///
    /// # Errors
    /// Returns `HandshakeError` if any connection or handshake fails.
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
    /// When the `PeerIPC` value is dropped we abort the spawned worker task.
    ///
    /// This avoids leaving a background task running after the wrapper is
    /// dropped, and logs a debug message with the peer hostname.
    fn drop(&mut self) {
        log::debug!("Dropping PeerIPC {}", self.hostname);
        self.thread.abort();
    }
}
