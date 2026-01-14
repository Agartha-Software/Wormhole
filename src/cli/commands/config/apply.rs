use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::{
        connection::{recieve_answer, send_command},
        ConfigType, IdentifyPodAndConfigArgs,
    },
    ipc::{
        answers::{ApplyConfigAnswer, ConfigFileError},
        commands::{Command, PodId},
    },
};

pub async fn apply(args: IdentifyPodAndConfigArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args.group);

    send_command(
        Command::ApplyConfig(pod, args.config_type.clone()),
        &mut stream,
    )
    .await?;

    match recieve_answer::<ApplyConfigAnswer>(&mut stream).await? {
        ApplyConfigAnswer::Success if args.config_type == ConfigType::Local => Ok("Local configuration applied!".into()),
        ApplyConfigAnswer::Success if args.config_type == ConfigType::Global => Ok("Global configuration applied!".into()),
        ApplyConfigAnswer::Success => Ok("Configuration applied!".into()),
        ApplyConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        ApplyConfigAnswer::ConfigFileError(ConfigFileError::MissingGlobal) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a `global_config.toml` file to check.",
        )),
        ApplyConfigAnswer::ConfigFileError(ConfigFileError::MissingLocal) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a `local_config.toml` file to check.",
        )),
        ApplyConfigAnswer::ConfigFileError(ConfigFileError::MissingBoth) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a either a `local_config.toml` or a `global_config.toml` file to check.",
        )),
        ApplyConfigAnswer::ConfigFileError(ConfigFileError::InvalidGlobal(error)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Global configuration validation failed:\n{error}"),
        )),
        ApplyConfigAnswer::ConfigFileError(ConfigFileError::InvalidLocal(error)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Local configuration validation failed:\n{error}"),
        )),
        ApplyConfigAnswer::ConfigFileError(ConfigFileError::InvalidBoth(local_error, global_error)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Local configuration validation failed:\n{local_error}\n\nGlobal configuration validation failed:\n{global_error}"),
        )),
    }
}
