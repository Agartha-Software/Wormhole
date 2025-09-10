use futures_util::SinkExt;
use futures_util::{stream::SplitStream, Sink, StreamExt};
use log::{error, warn};
use std::fmt::Debug;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

use crate::error::WhError;
use crate::network::message::MessageContent;

use super::message::{FromNetworkMessage, MessageAndStatus};

pub async fn forward_sender_to_peer<T>(
    mut peer_write: T,
    sender_out: &mut UnboundedReceiver<MessageAndStatus>,
    peer: String,
) where
    T: Sink<Message> + Unpin,
    <T as Sink<Message>>::Error: Debug,
{
    while let Some((message, status_tx)) = sender_out.recv().await {
        let serialized = bincode::serialize::<MessageContent>(&message);
        match serialized {
            Err(e) => error!("failed to serialize toward {peer}: {e:?}"),
            Ok(serialized) => {
                let sent = peer_write.send(Message::binary(serialized)).await;

                if let Err(e) = &sent {
                    error!("failed to send to {peer}: {e:?}");
                }

                status_tx.inspect(|tx| {
                    let _ = tx.send(sent.map_err(|_| WhError::NetworkDied {
                        called_from: "forward_sender_to_write".to_string(),
                    }));
                });
            }
        }
    }
}

pub async fn forward_peer_to_receiver<
    T: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>,
>(
    mut peer_read: SplitStream<T>,
    receiver_in: UnboundedSender<FromNetworkMessage>,
    peer: String,
) {
    while let Some(message) = peer_read.next().await {
        match message {
            Err(e) => error!("failed to read from {peer}: {e:?}"),
            Ok(Message::Binary(message)) => {
                let deserialized = bincode::deserialize::<MessageContent>(&message);
                match deserialized {
                    Err(e) => error!("failed to deserialize from {peer}: {e:?}"),
                    Ok(deserialized) => {
                        let sent = receiver_in.send(FromNetworkMessage {
                            origin: peer.clone(),
                            content: deserialized,
                        });
                        if let Err(e) = sent {
                            error!("failed to forward message from {peer}: {e:?}");
                        }
                    }
                }
            }
            other => warn!("unexpected message from {peer}: {other:?}"),
        }
    }

    log::warn!("forward from {peer} closed by remote host");
}
