use std::path::PathBuf;

use crate::cli::ConfigType;
use crate::config::local_file::LocalConfigFile;
use crate::config::types::Config;
use crate::config::GlobalConfig;
use crate::ipc::{answers::GenerateConfigAnswer, commands::PodId};
use crate::pods::itree::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME};
use crate::pods::pod::Pod;
use crate::service::Service;
use crate::service::{commands::find_pod, connection::send_answer};

fn write_defaults(
    path: PathBuf,
    config_type: ConfigType,
    overwrite: bool,
) -> Result<(), GenerateConfigAnswer> {
    if config_type.is_local() {
        let mut local_path = path.clone();
        local_path.push(LOCAL_CONFIG_FNAME);

        if !overwrite && local_path.exists() {
            return Err(GenerateConfigAnswer::CantOverwrite(ConfigType::Local));
        }
        LocalConfigFile::default()
            .write(local_path)
            .map_err(|err| GenerateConfigAnswer::WriteFailed(err.to_string(), ConfigType::Local))?;
    }

    if config_type.is_global() {
        let mut global_path = path.clone();
        global_path.push(GLOBAL_CONFIG_FNAME);

        if !overwrite && global_path.exists() {
            return Err(GenerateConfigAnswer::CantOverwrite(ConfigType::Global));
        }
        GlobalConfig::default().write(global_path).map_err(|err| {
            GenerateConfigAnswer::WriteFailed(err.to_string(), ConfigType::Global)
        })?;
    }
    Ok(())
}

// Return true no error have been sent and the command can continue
async fn write_local_config<Stream>(
    pod: &Pod,
    stream: &mut either::Either<&mut Stream, &mut String>,
    overwrite: bool,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let mut local_path = pod.get_mountpoint().clone();
    local_path.push(LOCAL_CONFIG_FNAME);

    if !overwrite && local_path.exists() {
        send_answer(
            GenerateConfigAnswer::CantOverwrite(ConfigType::Local),
            stream,
        )
        .await?;
        return Ok(false);
    }

    if let Err(err) = pod.generate_local_config().write(local_path) {
        send_answer(
            GenerateConfigAnswer::WriteFailed(err.to_string(), ConfigType::Local),
            stream,
        )
        .await?;
        return Ok(false);
    };
    Ok(true)
}

// Return true no error have been sent and the command can continue
async fn write_global_config<Stream>(
    pod: &Pod,
    stream: &mut either::Either<&mut Stream, &mut String>,
    overwrite: bool,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    if let Ok(config) = GlobalConfig::read_lock(&pod.global_config, "Write global config command") {
        let mut global_path = pod.get_mountpoint().clone();
        global_path.push(GLOBAL_CONFIG_FNAME);

        if !overwrite && global_path.exists() {
            send_answer(
                GenerateConfigAnswer::CantOverwrite(ConfigType::Global),
                stream,
            )
            .await?;
            return Ok(false);
        }

        if let Err(err) = config.write(global_path) {
            send_answer(
                GenerateConfigAnswer::WriteFailed(err.to_string(), ConfigType::Global),
                stream,
            )
            .await?;
            return Ok(false);
        };
    } else {
        send_answer(GenerateConfigAnswer::ConfigBlock, stream).await?;
        return Ok(false);
    };
    Ok(true)
}

impl Service {
    pub async fn generate<Stream>(
        &self,
        args: PodId,
        overwrite: bool,
        config_type: ConfigType,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        match find_pod(&args, &self.pods) {
            Some((_, pod)) => match config_type {
                ConfigType::Local => {
                    if write_local_config(pod, stream, overwrite).await? {
                        send_answer(GenerateConfigAnswer::Success, stream).await?;
                    }
                }
                ConfigType::Global => {
                    if write_global_config(pod, stream, overwrite).await? {
                        send_answer(GenerateConfigAnswer::Success, stream).await?;
                    }
                }
                ConfigType::Both => {
                    if write_local_config(pod, stream, overwrite).await? {
                        if write_global_config(pod, stream, overwrite).await? {
                            send_answer(GenerateConfigAnswer::Success, stream).await?;
                        }
                    }
                }
            },
            None => match args {
                PodId::Name(_) => send_answer(GenerateConfigAnswer::PodNotFound, stream).await?,
                PodId::Path(path) => {
                    if path.exists() {
                        if !path.is_dir() {
                            send_answer(GenerateConfigAnswer::NotADirectory, stream).await?;
                        }
                    } else if let Err(err) = std::fs::create_dir(&path) {
                        send_answer(
                            GenerateConfigAnswer::WriteFailed(err.to_string(), ConfigType::Both),
                            stream,
                        )
                        .await?;
                    }

                    match write_defaults(path, config_type, overwrite) {
                        Ok(()) => send_answer(GenerateConfigAnswer::SuccessDefault, stream).await?,
                        Err(err) => send_answer(err, stream).await?,
                    }
                }
            },
        }
        Ok(())
    }
}
