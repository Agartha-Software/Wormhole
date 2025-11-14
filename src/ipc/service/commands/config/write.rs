use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::types::Config;
use crate::config::{GlobalConfig, LocalConfig};
use crate::ipc::service::commands::find_pod;
use crate::ipc::{answers::WriteConfigAnswer, commands::PodId, service::connection::send_answer};
use crate::pods::pod::Pod;

fn write_defaults(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut local_path = path.clone();
    local_path.push(".local_config.toml");
    LocalConfig::default().write(local_path)?;

    let mut global_path = path.clone();
    global_path.push(".global_config.toml");
    GlobalConfig::default().write(global_path)?;
    Ok(())
}

// Return true no error have been sent and the command can continue
async fn write_local_config<Stream>(
    pod: &Pod,
    stream: &mut Stream,
    overwrite: bool,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    if let Ok(config) = LocalConfig::read_lock(&pod.local_config, "Write local config command") {
        let mut local_path = pod.get_mountpoint().clone();
        local_path.push(".local_config.toml");

        if !overwrite && PathBuf::from(local_path.to_string()).exists() {
            send_answer(WriteConfigAnswer::CantOverwrite, stream).await?;
            return Ok(false);
        }

        if let Err(err) = config.write(local_path) {
            send_answer(WriteConfigAnswer::WriteFailed(err.to_string()), stream).await?;
            return Ok(false);
        };
    } else {
        send_answer(WriteConfigAnswer::ConfigBlock, stream).await?;
        return Ok(false);
    };
    Ok(true)
}

// Return true no error have been sent and the command can continue
async fn write_global_config<Stream>(
    pod: &Pod,
    stream: &mut Stream,
    overwrite: bool,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    if let Ok(config) = GlobalConfig::read_lock(&pod.global_config, "Write global config command") {
        let mut global_path = pod.get_mountpoint().clone();
        global_path.push(".global_config.toml");

        if !overwrite && PathBuf::from(global_path.to_string()).exists() {
            send_answer(WriteConfigAnswer::CantOverwrite, stream).await?;
            return Ok(false);
        }

        if let Err(err) = config.write(global_path) {
            send_answer(WriteConfigAnswer::WriteFailed(err.to_string()), stream).await?;
            return Ok(false);
        };
    } else {
        send_answer(WriteConfigAnswer::ConfigBlock, stream).await?;
        return Ok(false);
    };
    Ok(true)
}

pub async fn write<Stream>(
    args: PodId,
    overwrite: bool,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => {
            if write_local_config(pod, stream, overwrite).await? {
                if write_global_config(pod, stream, overwrite).await? {
                    send_answer(WriteConfigAnswer::Success, stream).await?;
                }
            }
        }
        None => match args {
            PodId::Name(_) => send_answer(WriteConfigAnswer::PodNotFound, stream).await?,
            PodId::Path(path) => {
                if path.exists() {
                    if !path.is_dir() {
                        send_answer(WriteConfigAnswer::NotADirectory, stream).await?;
                        return Ok(false);
                    }
                } else if let Err(err) = std::fs::create_dir(&path) {
                    send_answer(WriteConfigAnswer::WriteFailed(err.to_string()), stream).await?;
                    return Ok(false);
                }

                match write_defaults(path) {
                    Ok(()) => send_answer(WriteConfigAnswer::Success, stream).await?,
                    Err(err) => {
                        send_answer(WriteConfigAnswer::WriteFailed(err.to_string()), stream).await?
                    }
                }
            }
        },
    };

    Ok(false)
}
