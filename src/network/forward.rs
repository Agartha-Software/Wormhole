use futures_util::SinkExt;
use futures_util::{stream::SplitStream, Sink, StreamExt};
use std::fmt::Debug;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

use crate::error::WhError;
use crate::network::message::MessageContent;

use super::message::{FromNetworkMessage, MessageAndStatus};

pub async fn forward_receiver_to_write<T>(
    mut write: T,
    rx: &mut UnboundedReceiver<MessageAndStatus>,
) where
    T: Sink<Message> + Unpin,
    <T as Sink<Message>>::Error: Debug,
{
    while let Some((message, status_tx)) = rx.recv().await {
        let serialized = bincode::serialize(&message).unwrap();
        let sent = write.send(Message::binary(serialized)).await;

        status_tx.inspect(|tx| {
            let _ = tx.send(sent.map_err(|_| WhError::NetworkDied {
                called_from: "forward_receiver_to_write".to_string(),
            }));
        });
    }
}

pub async fn forward_read_to_sender<
    T: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>,
>(
    mut read: SplitStream<T>,
    tx: UnboundedSender<FromNetworkMessage>,
    address: String,
) {
    while let Ok(Message::Binary(message)) = read.next().await.unwrap() {
        let deserialized: MessageContent = bincode::deserialize(&message).unwrap();
        tx.send(FromNetworkMessage {
            origin: address.clone(),
            content: deserialized,
        })
        .unwrap();
    }
}
