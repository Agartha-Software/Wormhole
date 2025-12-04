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

async fn locked_config_to_string<Conf>(
    config: &Arc<RwLock<Conf>>,
) -> std::io::Result<Option<String>>
where
    Conf: Serialize + Clone,
{
    let config = match config.try_read_for(LOCK_TIMEOUT) {
        Some(config) => config,
        None => return Ok(None),
    };

    Ok(Some(
        toml::to_string(&config.clone()).expect("Serialization shouldn't be able to fail"),
    ))
}

fn config_to_string<Conf>(config: &Conf) -> String
where
    Conf: Serialize + Clone,
{
    toml::to_string(&config.clone()).expect("Serialization shouldn't be able to fail")
}

pub async fn show<Stream>(
    args: PodId,
    config_type: ConfigType,
    pods: &HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => {
            match config_type {
                ConfigType::Local => {
                    send_answer(
                        ShowConfigAnswer::SuccessLocal(config_to_string(&pod.local_config)),
                        stream,
                    )
                    .await?
                }
                ConfigType::Global => match locked_config_to_string(&pod.global_config).await? {
                    Some(global) => {
                        send_answer(ShowConfigAnswer::SuccessGlobal(global), stream).await?
                    }
                    None => send_answer(ShowConfigAnswer::ConfigBlock, stream).await?,
                },
                ConfigType::Both => match locked_config_to_string(&pod.global_config).await? {
                    Some(global) => {
                        send_answer(
                            ShowConfigAnswer::SuccessBoth(
                                config_to_string(&pod.local_config),
                                global,
                            ),
                            stream,
                        )
                        .await?
                    }
                    None => send_answer(ShowConfigAnswer::ConfigBlock, stream).await?,
                },
            };
        }
        None => send_answer(ShowConfigAnswer::PodNotFound, stream).await?,
    };
    Ok(false)
}
