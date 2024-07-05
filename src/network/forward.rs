use futures_util::SinkExt;
use futures_util::{stream::SplitStream, Sink, StreamExt};
use tokio::sync::broadcast::{Receiver, Sender};
use std::fmt::Debug;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

use super::message::NetworkMessage;

pub async fn forward_receiver_to_write<T>(mut write: T, rx: &mut UnboundedReceiver<NetworkMessage>)
where
    T: Sink<Message> + Unpin,
    <T as Sink<Message>>::Error: Debug,
{
    while let Some(message) = rx.recv().await {
        let serialized = bincode::serialize(&message).unwrap();
        write.send(Message::binary(serialized)).await.unwrap();
    }
}

pub async fn forward_read_to_sender<
    T: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>,
>(
    mut read: SplitStream<T>,
    tx: UnboundedSender<NetworkMessage>,
) {
    while let Ok(Message::Binary(message)) = read.next().await.unwrap() {
        let deserialized = bincode::deserialize(&message).unwrap();
        tx.send(deserialized).unwrap();
    }
}
