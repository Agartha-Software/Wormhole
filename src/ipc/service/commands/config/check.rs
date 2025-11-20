use std::{collections::HashMap, path::PathBuf};

use crate::{
    cli::ConfigType,
    config::{types::Config, GlobalConfig, LocalConfig},
    ipc::{
        answers::CheckConfigAnswer,
        commands::PodId,
        service::{commands::find_pod, connection::send_answer},
    },
    pods::pod::Pod,
};

pub async fn check<Stream>(
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
            let mut local_path = pod.get_mountpoint().clone();
            local_path.push(".local_config.toml");
            let local_pathbuf = PathBuf::from(local_path.to_string());

            let mut global_path = pod.get_mountpoint().clone();
            global_path.push(".global_config.toml");
            let global_pathbuf = PathBuf::from(global_path.to_string());

            match (
                local_pathbuf.exists() || !config_type.is_local(),
                global_pathbuf.exists() || !config_type.is_global(),
            ) {
                (true, true) => (),
                (false, true) => send_answer(CheckConfigAnswer::MissingLocal, stream).await?,
                (true, false) => send_answer(CheckConfigAnswer::MissingGlobal, stream).await?,
                (false, false) => send_answer(CheckConfigAnswer::MissingBoth, stream).await?,
            }

            match (
                LocalConfig::read(local_path)
                    .err()
                    .filter(|_| config_type.is_local()),
                GlobalConfig::read(global_path)
                    .err()
                    .filter(|_| config_type.is_global()),
            ) {
                (None, None) => send_answer(CheckConfigAnswer::Success, stream),
                (Some(local_err), None) => send_answer(
                    CheckConfigAnswer::InvalidLocal(local_err.to_string()),
                    stream,
                ),
                (None, Some(global_err)) => send_answer(
                    CheckConfigAnswer::InvalidGlobal(global_err.to_string()),
                    stream,
                ),
                (Some(local_err), Some(global_err)) => send_answer(
                    CheckConfigAnswer::InvalidBoth(local_err.to_string(), global_err.to_string()),
                    stream,
                ),
            }
            .await?;
        }
        None => send_answer(CheckConfigAnswer::PodNotFound, stream).await?,
    }
    Ok(false)
}
