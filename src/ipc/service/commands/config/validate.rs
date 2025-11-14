use std::{collections::HashMap, path::PathBuf};

use crate::{
    config::{types::Config, GlobalConfig, LocalConfig},
    ipc::{
        answers::ValidateConfigAnswer,
        commands::PodId,
        service::{commands::find_pod, connection::send_answer},
    },
    pods::pod::Pod,
};

pub async fn validate<Stream>(
    args: PodId,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => {
            let mut local_path = pod.get_mountpoint().clone();
            local_path.push("local_config.toml");
            let local_pathbuf = PathBuf::from(local_path.to_string());

            let mut global_path = pod.get_mountpoint().clone();
            global_path.push("global_config.toml");
            let global_pathbuf = PathBuf::from(global_path.to_string());

            match (local_pathbuf.exists(), global_pathbuf.exists()) {
                (true, true) => (),
                (true, false) => send_answer(ValidateConfigAnswer::MissingLocal, stream).await?,
                (false, true) => send_answer(ValidateConfigAnswer::MissingGlobal, stream).await?,
                (false, false) => send_answer(ValidateConfigAnswer::MissingBoth, stream).await?,
            }

            match (
                LocalConfig::read(local_path),
                GlobalConfig::read(global_path),
            ) {
                (Ok(_), Ok(_)) => send_answer(ValidateConfigAnswer::Success, stream),
                (Err(local_err), Ok(_)) => send_answer(
                    ValidateConfigAnswer::InvalidLocal(local_err.to_string()),
                    stream,
                ),
                (Ok(_), Err(global_err)) => send_answer(
                    ValidateConfigAnswer::InvalidGlobal(global_err.to_string()),
                    stream,
                ),
                (Err(local_err), Err(global_err)) => send_answer(
                    ValidateConfigAnswer::InvalidBoth(
                        local_err.to_string(),
                        global_err.to_string(),
                    ),
                    stream,
                ),
            }
            .await?;
        }
        None => send_answer(ValidateConfigAnswer::PodNotFound, stream).await?,
    }
    Ok(false)
}
