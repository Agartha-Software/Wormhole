use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::{ConfigType, SaveConfigArgs},
    ipc::{
        answers::SaveConfigAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn save(args: SaveConfigArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args.group);

    send_command(
        Command::SaveConfig(pod, args.force, args.config_type),
        &mut stream,
    )
    .await?;

    match recieve_answer::<SaveConfigAnswer>(&mut stream).await? {
        SaveConfigAnswer::Success => Ok("Pod's configuration created successfully!".into()),
        SaveConfigAnswer::SuccessDefault => Ok("Default configuration created successfully!".into()),
        SaveConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        SaveConfigAnswer::NotADirectory => Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "The given directory doesn't exist.",
        )),
        SaveConfigAnswer::ConfigBlock => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Failed to access the pod configuration.",
        )),
        SaveConfigAnswer::WriteFailed(err, ConfigType::Local) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the local configuration: {err}"),
        )),
        SaveConfigAnswer::WriteFailed(err, ConfigType::Global) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the global configuration: {err}"),
        )),
        SaveConfigAnswer::WriteFailed(err, ConfigType::Both) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the local configuration: {err}"),
        )),
        SaveConfigAnswer::CantOverwrite(ConfigType::Local) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the local configuration file, it already exist...\n`--overwrite` to overwrite existing files"
            ),
        )),
        SaveConfigAnswer::CantOverwrite(ConfigType::Global) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the global configuration file, it already exist...\n`--overwrite` to overwrite existing files"
            ),
        )),
        SaveConfigAnswer::CantOverwrite(ConfigType::Both) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the configuration files, they already exist...\n`--overwrite` to overwrite existing files"
            ),
        )),
    }
}
