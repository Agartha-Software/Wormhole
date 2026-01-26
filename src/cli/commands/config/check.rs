use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::{
        connection::{recieve_answer, send_command},
        ConfigType, IdentifyPodAndConfigArgs,
    },
    ipc::{
        answers::{CheckConfigAnswer, ConfigFileError},
        commands::{Command, PodId},
    },
};

pub async fn check(args: IdentifyPodAndConfigArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args.group);

    send_command(
        Command::CheckConfig(pod, args.config_type.clone()),
        &mut stream,
    )
    .await?;

    match recieve_answer::<CheckConfigAnswer>(&mut stream).await? {
        CheckConfigAnswer::Success if args.config_type == ConfigType::Local => Ok("The pod local configuration file is valid!".into()),
        CheckConfigAnswer::Success if args.config_type == ConfigType::Global => Ok("The pod global configuration file is valid!".into()),
        CheckConfigAnswer::Success => Ok("The pod configuration files are valid!".into()),
        CheckConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        CheckConfigAnswer::ConfigFileError(ConfigFileError::MissingGlobal) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a `global_config.toml` file to check.",
        )),
        CheckConfigAnswer::ConfigFileError(ConfigFileError::MissingLocal) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a `local_config.toml` file to check.",
        )),
        CheckConfigAnswer::ConfigFileError(ConfigFileError::MissingBoth) => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a either a `local_config.toml` or a `global_config.toml` file to check.",
        )),
        CheckConfigAnswer::ConfigFileError(ConfigFileError::InvalidGlobal(error)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Global configuration validation failed:\n{error}"),
        )),
        CheckConfigAnswer::ConfigFileError(ConfigFileError::InvalidLocal(error)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Local configuration validation failed:\n{error}"),
        )),
        CheckConfigAnswer::ConfigFileError(ConfigFileError::InvalidBoth(local_error, global_error)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Local configuration validation failed:\n{local_error}\n\nGlobal configuration validation failed:\n{global_error}"),
        )),
    }
}
