use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::{ConfigType, WriteConfigArgs},
    ipc::{
        answers::WriteConfigAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn write(args: WriteConfigArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args.group);

    send_command(
        Command::WriteConfg(pod, args.overwrite, args.config_type),
        &mut stream,
    )
    .await?;

    match recieve_answer::<WriteConfigAnswer>(&mut stream).await? {
        WriteConfigAnswer::Success => Ok("Pod's configuration created successfully!".into()),
        WriteConfigAnswer::SuccessDefault => Ok("Default configuration created successfully!".into()),
        WriteConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        WriteConfigAnswer::NotADirectory => Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "The given directory doesn't exist.",
        )),
        WriteConfigAnswer::ConfigBlock => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Failed to access the pod configuration.",
        )),
        WriteConfigAnswer::WriteFailed(err, ConfigType::Local) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the local configuration: {err}"),
        )),
        WriteConfigAnswer::WriteFailed(err, ConfigType::Global) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the global configuration: {err}"),
        )),
        WriteConfigAnswer::WriteFailed(err, ConfigType::Both) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the local configuration: {err}"),
        )),
        WriteConfigAnswer::CantOverwrite(ConfigType::Local) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the local configuration file, it already exist...\n`--overwrite` to overwrite existing files"
            ),
        )),
        WriteConfigAnswer::CantOverwrite(ConfigType::Global) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the global configuration file, it already exist...\n`--overwrite` to overwrite existing files"
            ),
        )),
        WriteConfigAnswer::CantOverwrite(ConfigType::Both) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write the configuration files, they already exist...\n`--overwrite` to overwrite existing files"
            ),
        )),
    }
}
