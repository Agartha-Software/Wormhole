use std::collections::HashMap;

use crate::{
    config::{types::Config, GlobalConfig, LocalConfig},
    ipc::{
        answers::ShowConfigAnswer,
        commands::PodId,
        service::{commands::find_pod, connection::send_answer},
    },
    pods::pod::Pod,
};

pub async fn show<Stream>(
    args: PodId,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => {
            let global_config = match GlobalConfig::read_lock(&pod.global_config, "show config") {
                Ok(global_config) => global_config,
                Err(_) => {
                    send_answer(ShowConfigAnswer::ConfigBlock, stream).await?;
                    return Ok(false);
                }
            };

            match LocalConfig::read_lock(&pod.local_config, "show config") {
                Ok(local_config) => {
                    let global_str = toml::to_string(&global_config.clone())
                        .expect("Serialization should'nt be able to fail");
                    let local_str = toml::to_string(&local_config.clone())
                        .expect("Serialization should'nt be able to fail");

                    send_answer(
                        ShowConfigAnswer::Success(format!("{global_str}\n{local_str}",)),
                        stream,
                    )
                    .await?;
                }
                Err(_) => {
                    send_answer(ShowConfigAnswer::ConfigBlock, stream).await?;
                }
            };
        }
        None => send_answer(ShowConfigAnswer::PodNotFound, stream).await?,
    };
    Ok(false)
}
