use either::Either;
use interprocess::local_socket;
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;

use crate::ipc::commands::Command;
use crate::service::Service;

impl Service {
    pub async fn handle_connection<Stream>(&mut self, mut stream: Stream) -> bool
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        log::trace!("Connection from CLI recieved!");

        let size = stream
            .read_u32()
            .await
            .expect("Failed to read recieved command, shouldn't be possible!");
        let mut buffer: Vec<u8> = vec![0; size as usize];
        let _size = stream
            .read_exact(&mut buffer)
            .await
            .expect("Failed to read recieved command, shouldn't be possible!");

        match bincode::deserialize::<Command>(&buffer) {
            Ok(command) => self
                .handle_command(command, &mut Either::Left(&mut stream))
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

    /// Borrow pods, execute the command, sends the answer to the web api
    pub async fn handle_tcp_connection(
        &mut self,
        command: Command,
        reply_tx: oneshot::Sender<String>,
    ) -> bool {
        let mut buffer = String::new();
        let mut stream = Either::Right(&mut buffer);
        let _ = self
            .handle_command::<local_socket::tokio::Stream>(command, &mut stream)
            .await;
        let _ = reply_tx.send(buffer);
        false
    }

    pub async fn handle_command<Stream>(
        &mut self,
        command: Command,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<bool>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let stop = false;

        match command {
            Command::Unfreeze(pod_id) => self.unfreeze(pod_id, stream).await,
            Command::Freeze(pod_id) => self.freeze(pod_id, stream).await,
            Command::Restart(pod_id) => self.restart(pod_id, stream).await,
            Command::Status => self.status(stream).await,
            Command::New(request) => self.new_command(request, stream).await,
            Command::GetHosts(request) => self.gethosts(request, stream).await,
            Command::Inspect(pod_id) => self.inspect(pod_id, stream).await,
            Command::Metrics(pod_id) => self.metrics(pod_id, stream).await,
            Command::Remove(request) => self.remove(request, stream).await,
            Command::Tree(pod_id) => self.tree(pod_id, stream).await,
            Command::GenerateConfig(pod_id, overwrite, config_type) => {
                self.generate(pod_id, overwrite, config_type, stream).await
            }
            Command::ShowConfig(pod_id, config_type) => {
                self.show(pod_id, config_type, stream).await
            }
            Command::CheckConfig(pod_id, config_type) => {
                self.check(pod_id, config_type, stream).await
            }
            Command::RedundancyStatus(pod_id) => self.redundancy_status(pod_id, stream).await,
            Command::StatsPerFiletype(pod_id) => self.stats_per_filetype(pod_id, stream).await,
            Command::ListPods => self.list_pods(stream).await,
        }?;
        Ok(stop)
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
            stream.write_all(&serialized).await?;
            stream.flush().await
        }
        Either::Right(stream) => {
            let serialized = serde_json::to_value(answer)?;
            **stream = serialized.to_string();
            Ok(())
        }
    }
}
