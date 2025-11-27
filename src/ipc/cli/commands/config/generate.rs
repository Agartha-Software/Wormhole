use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::{ConfigType, GenerateConfigArgs},
    ipc::{
        answers::GenerateConfigAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn generate(args: GenerateConfigArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args.group);

    send_command(
        Command::GenerateConfig(pod, args.force, args.config_type),
        &mut stream,
    )
    .await?;

    match recieve_answer::<GenerateConfigAnswer>(&mut stream).await? {
        GenerateConfigAnswer::Success => Ok("Pod's configuration created successfully!".into()),
        GenerateConfigAnswer::SuccessDefault => Ok("Default configuration created successfully!".into()),
        GenerateConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        GenerateConfigAnswer::NotADirectory => Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "The given directory doesn't exist.",
        )),
        GenerateConfigAnswer::ConfigBlock => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Failed to access the pod configuration.",
        )),
        GenerateConfigAnswer::WriteFailed(err, ConfigType::Local) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the local configuration: {err}"),
        )),
        GenerateConfigAnswer::WriteFailed(err, ConfigType::Global) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the global configuration: {err}"),
        )),
        GenerateConfigAnswer::WriteFailed(err, ConfigType::Both) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the local configuration: {err}"),
        )),
        GenerateConfigAnswer::CantOverwrite(ConfigType::Local) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the local configuration file, it already exist...\n`--force` to overwrite existing files"
            ),
        )),
        GenerateConfigAnswer::CantOverwrite(ConfigType::Global) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the global configuration file, it already exist...\n`--force` to overwrite existing files"
            ),
        )),
        GenerateConfigAnswer::CantOverwrite(ConfigType::Both) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the configuration files, they already exist...\n`--force` to overwrite existing files"
            ),
        )),
    }
}
