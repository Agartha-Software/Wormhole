use std::collections::HashMap;

use crate::{
    cli::ConfigType,
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
    config_type: ConfigType,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => {
            let local_config = match LocalConfig::read_lock(&pod.local_config, "show config") {
                Ok(local_config) => local_config,
                Err(_) => {
                    send_answer(ShowConfigAnswer::ConfigBlock, stream).await?;
                    return Ok(false);
                }
            };

            match GlobalConfig::read_lock(&pod.global_config, "show config") {
                Ok(global_config) => {
                    let local_str = toml::to_string(&local_config.clone())
                        .expect("Serialization should'nt be able to fail");
                    let global_str = toml::to_string(&global_config.clone())
                        .expect("Serialization should'nt be able to fail");

                    send_answer(ShowConfigAnswer::Success(local_str, global_str), stream).await?;
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
