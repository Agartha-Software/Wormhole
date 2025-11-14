use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::ConfigType;
use crate::config::types::Config;
use crate::config::{GlobalConfig, LocalConfig};
use crate::ipc::service::commands::find_pod;
use crate::ipc::{answers::WriteConfigAnswer, commands::PodId, service::connection::send_answer};
use crate::pods::pod::Pod;

fn write_defaults(
    path: PathBuf,
    config_type: ConfigType,
    overwrite: bool,
) -> Result<(), WriteConfigAnswer> {
    if config_type.is_local() {
        let mut local_path = path.clone();
        local_path.push(".local_config.toml");

        if !overwrite && local_path.exists() {
            return Err(WriteConfigAnswer::CantOverwrite(ConfigType::Local));
        }
        LocalConfig::default()
            .write(local_path)
            .map_err(|err| WriteConfigAnswer::WriteFailed(err.to_string(), ConfigType::Local))?;
    }

    if config_type.is_global() {
        let mut global_path = path.clone();
        global_path.push(".global_config.toml");

        if !overwrite && global_path.exists() {
            return Err(WriteConfigAnswer::CantOverwrite(ConfigType::Global));
        }
        GlobalConfig::default()
            .write(global_path)
            .map_err(|err| WriteConfigAnswer::WriteFailed(err.to_string(), ConfigType::Global))?;
    }
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
            send_answer(WriteConfigAnswer::CantOverwrite(ConfigType::Local), stream).await?;
            return Ok(false);
        }

        if let Err(err) = config.write(local_path) {
            send_answer(
                WriteConfigAnswer::WriteFailed(err.to_string(), ConfigType::Local),
                stream,
            )
            .await?;
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
            send_answer(WriteConfigAnswer::CantOverwrite(ConfigType::Global), stream).await?;
            return Ok(false);
        }

        if let Err(err) = config.write(global_path) {
            send_answer(
                WriteConfigAnswer::WriteFailed(err.to_string(), ConfigType::Global),
                stream,
            )
            .await?;
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
    config_type: ConfigType,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => match config_type {
            ConfigType::Local => {
                if write_local_config(pod, stream, overwrite).await? {
                    send_answer(WriteConfigAnswer::Success, stream).await?;
                }
            }
            ConfigType::Global => {
                if write_global_config(pod, stream, overwrite).await? {
                    send_answer(WriteConfigAnswer::Success, stream).await?;
                }
            }
            ConfigType::Both => {
                if write_local_config(pod, stream, overwrite).await? {
                    if write_global_config(pod, stream, overwrite).await? {
                        send_answer(WriteConfigAnswer::Success, stream).await?;
                    }
                }
            }
        },
        None => match args {
            PodId::Name(_) => send_answer(WriteConfigAnswer::PodNotFound, stream).await?,
            PodId::Path(path) => {
                if path.exists() {
                    if !path.is_dir() {
                        send_answer(WriteConfigAnswer::NotADirectory, stream).await?;
                        return Ok(false);
                    }
                } else if let Err(err) = std::fs::create_dir(&path) {
                    send_answer(
                        WriteConfigAnswer::WriteFailed(err.to_string(), ConfigType::Both),
                        stream,
                    )
                    .await?;
                    return Ok(false);
                }

                match write_defaults(path, config_type, overwrite) {
                    Ok(()) => send_answer(WriteConfigAnswer::SuccessDefault, stream).await?,
                    Err(err) => send_answer(err, stream).await?,
                }
            }
        },
    };

    Ok(false)
}
