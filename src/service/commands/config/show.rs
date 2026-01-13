use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;
use serde::Serialize;

use crate::{
    cli::ConfigType,
    ipc::{answers::ShowConfigAnswer, commands::PodId},
    pods::{itree::LOCK_TIMEOUT, pod::Pod},
    service::{commands::find_pod, connection::send_answer},
};

async fn locking_config_to_string<Conf>(
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

fn generate_local_config_file(pod: &Pod) -> String {
    let config = pod.generate_local_config();
    toml::to_string(&config).expect("Serialization shouldn't be able to fail")
}

pub async fn show<Stream>(
    args: PodId,
    config_type: ConfigType,
    pods: &HashMap<String, Pod>,
    stream: &mut either::Either<&mut Stream, &mut String>,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => {
            match config_type {
                ConfigType::Local => {
                    send_answer(
                        ShowConfigAnswer::SuccessLocal(generate_local_config_file(pod)),
                        stream,
                    )
                    .await?
                }
                ConfigType::Global => match locking_config_to_string(&pod.global_config).await? {
                    Some(global) => {
                        send_answer(ShowConfigAnswer::SuccessGlobal(global), stream).await?
                    }
                    None => send_answer(ShowConfigAnswer::ConfigBlock, stream).await?,
                },
                ConfigType::Both => match locking_config_to_string(&pod.global_config).await? {
                    Some(global) => {
                        send_answer(
                            ShowConfigAnswer::SuccessBoth(generate_local_config_file(pod), global),
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
