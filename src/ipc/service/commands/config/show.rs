use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;
use serde::Serialize;

use crate::{
    cli::ConfigType,
    ipc::{
        answers::ShowConfigAnswer,
        commands::PodId,
        service::{commands::find_pod, connection::send_answer},
    },
    pods::{arbo::LOCK_TIMEOUT, pod::Pod},
};

async fn config_as_str<Conf, Stream>(
    config: &Arc<RwLock<Conf>>,
    stream: &mut Stream,
) -> std::io::Result<Option<String>>
where
    Conf: Serialize + Clone,
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let config = match config.try_read_for(LOCK_TIMEOUT) {
        Some(config) => config,
        None => {
            send_answer(ShowConfigAnswer::ConfigBlock, stream).await?;
            return Ok(None);
        }
    };

    Ok(Some(
        toml::to_string(&config.clone()).expect("Serialization should'nt be able to fail"),
    ))
}

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
            match config_type {
                ConfigType::Local => {
                    if let Some(local) = config_as_str(&pod.local_config, stream).await? {
                        send_answer(ShowConfigAnswer::SuccessLocal(local), stream).await?;
                    }
                }
                ConfigType::Global => {
                    if let Some(global) = config_as_str(&pod.global_config, stream).await? {
                        send_answer(ShowConfigAnswer::SuccessGlobal(global), stream).await?;
                    }
                }
                ConfigType::Both => {
                    if let Some(local) = config_as_str(&pod.local_config, stream).await? {
                        if let Some(global) = config_as_str(&pod.global_config, stream).await? {
                            send_answer(ShowConfigAnswer::SuccessBoth(local, global), stream)
                                .await?;
                        }
                    }
                }
            };
        }
        None => send_answer(ShowConfigAnswer::PodNotFound, stream).await?,
    };
    Ok(false)
}
