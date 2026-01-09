use std::collections::HashMap;

use crate::{
    ipc::{
        commands::Command,
        service::commands::{
            check, freeze, generate, gethosts, inspect, new, remove, show, status, tree, unfreeze,
        },
    },
    pods::pod::Pod,
};
use either::Either;
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

pub async fn handle_connection<Stream>(pods: &mut HashMap<String, Pod>, mut stream: Stream) -> bool
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    log::trace!("Connection from CLI recieved!");

    let size = stream
        .read_u32()
        .await
        .expect("Failed to read recieved command, shouldn't be possible!");
    let mut buffer: Vec<u8> = Vec::with_capacity(size as usize);
    let _size = stream
        .read_buf(&mut buffer)
        .await
        .expect("Failed to read recieved command, shouldn't be possible!");

    match bincode::deserialize::<Command>(&buffer) {
        Ok(command) => handle_command(command, pods, &mut Either::Left(&mut stream))
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

pub async fn send_answer<T, Stream>(
    answer: T,
    stream: &mut Either<&mut Stream, &mut String>,
) -> std::io::Result<()>
where
    T: Serialize,
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match stream {
        Either::Left(stream) => {
            let serialized = bincode::serialize(&answer)
                .expect("Can't serialize cli answer, shouldn't be possible!");

            stream.write_u32(serialized.len() as u32).await?;
            stream.write_all(&serialized).await
        }
        Either::Right(stream) => {
            let serialized = serde_json::to_value(answer)?;
            **stream = serialized.to_string();
            Ok(())
        }
    }
}

pub async fn handle_command<Stream>(
    command: Command,
    pods: &mut HashMap<String, Pod>,
    stream: &mut either::Either<&mut Stream, &mut String>,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match command {
        Command::Unfreeze(pod_id) => unfreeze(pod_id, stream).await,
        Command::Status => status(stream).await,
        Command::Freeze(pod_id) => freeze(pod_id, stream).await,
        Command::New(request) => new(request, pods, stream).await,
        Command::GetHosts(request) => gethosts(request, pods, stream).await,
        Command::Inspect(pod_id) => inspect(pod_id, pods, stream).await,
        Command::Remove(request) => remove(request, pods, stream).await,
        Command::Tree(pod_id) => tree(pod_id, pods, stream).await,
        Command::GenerateConfig(pod_id, overwrite, config_type) => {
            generate(pod_id, overwrite, config_type, pods, stream).await
        }
        Command::ShowConfig(pod_id, config_type) => show(pod_id, config_type, pods, stream).await,
        Command::CheckConfig(data, config_type) => check(data, config_type, pods, stream).await,
    }
}
