use std::collections::HashMap;

use crate::{
    ipc::{
        commands::Command,
        service::commands::{freeze, gethosts, new, unfreeze},
    },
    pods::pod::Pod,
};
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

pub async fn handle_connection<Stream>(pods: &mut HashMap<String, Pod>, mut stream: Stream) -> bool
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    log::debug!("Connection recieved");

    let size = stream
        .read_u32()
        .await
        .expect("Failed to read recieved command, shouldn't be possible!");
    log::trace!("waiting for {size} bytes!");
    let mut buffer: Vec<u8> = Vec::with_capacity(size as usize);
    let _size = stream
        .read_buf(&mut buffer)
        .await
        .expect("Failed to read recieved command, shouldn't be possible!");

    log::trace!("found {_size} bytes!");

    match bincode::deserialize::<Command>(&buffer) {
        Ok(command) => handle_command(command, pods, stream)
            .await
            .unwrap_or_else(|e| {
                log::error!("Network Error: {e:?}"); // TODO verify relevance
                false
            }),
        Err(err) => {
            log::error!("Command recieved not recognized by the service: {err:?}");
            eprintln!("Command recieved not recognized by the service.");
            false
        }
    }
}

pub async fn send_answer<T, Stream>(answer: T, stream: &mut Stream) -> std::io::Result<()>
where
    T: Serialize,
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let serialized =
        bincode::serialize(&answer).expect("Can't serialize cli answer, shouldn't be possible!");

    stream.write_u32(serialized.len() as u32).await?;
    stream.write_all(&serialized).await
}

async fn handle_command<Stream>(
    command: Command,
    pods: &mut HashMap<String, Pod>,
    mut stream: Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let stream = &mut stream;

    match command {
        Command::Unfreeze(data) => unfreeze(data, stream).await,
        Command::Freeze(data) => freeze(data, stream).await,
        Command::New(data) => new(data, pods, stream).await,
        Command::GetHosts(data) => gethosts(data, pods, stream).await,
    }
}
