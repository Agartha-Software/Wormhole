use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio_tungstenite::tungstenite::Message;
use std::{fmt::Debug, sync::Arc};

use futures_util::SinkExt;
use futures_util::{stream::SplitStream, Sink, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

// use crate::network::forward::{forward_read_to_sender2, forward_receiver_to_write2};

use super::{commands::publish_file, message::NetworkMessage, server::State};

pub struct PeerIPC {
    pub address: String,
    pub thread: tokio::task::JoinHandle<()>,
    pub sender: broadcast::Sender<NetworkMessage>,
    pub receiver: broadcast::Receiver<NetworkMessage>,
    state: Arc<State>,
}

async fn forward_receiver_to_write2<T>(mut write: T, rx: &mut Receiver<NetworkMessage>)
where
    T: Sink<Message> + Unpin,
    <T as Sink<Message>>::Error: Debug,
{
    while let Ok(message) = rx.recv().await {
        let serialized = bincode::serialize(&message).unwrap();
        write.send(Message::binary(serialized));
    }
}

async fn forward_read_to_sender2<
    T: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>,
>(state: &Arc<State>,
    mut read: SplitStream<T>,
    tx: Sender<NetworkMessage>,
    address: &String,
) {
    while let Ok(Message::Binary(message)) = read.next().await.unwrap() {
        let deserialized = bincode::deserialize(&message).unwrap();
        if let NetworkMessage::RequestFile(path) = deserialized {
            publish_file(state, &path.pod, &path.file, address).await;
        } else {
            tx.send(deserialized).unwrap();
        }
    }
}

impl PeerIPC {

    pub async fn work(state: Arc<State>, address: String, sender: broadcast::Sender<NetworkMessage>, mut receiver: broadcast::Receiver<NetworkMessage>) {
        if let Ok((stream, _)) = tokio_tungstenite::connect_async(&address).await {
            let (write, read) = stream.split();
            tokio::join!(
                forward_read_to_sender2(&state, read, sender, &address),
                forward_receiver_to_write2(write, &mut receiver)
            );
        }
    }

    pub fn connect(address: String, state: &Arc<State>) -> Self {
        let (outbound_send, outbound_recv) = broadcast::channel(16);
        let (inbound_send, inbound_recv) = broadcast::channel(16);
        Self {
            thread: tokio::spawn(Self::work(state.clone(), address.clone(), inbound_send, outbound_recv)),
            address,
            sender: outbound_send,
            receiver: inbound_recv,
            state: state.clone(),
        }
    }
}
