use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        answers::ValidateConfigAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn validate(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args);

    send_command(Command::ValidateConfig(pod), &mut stream).await?;

    match recieve_answer::<ValidateConfigAnswer>(&mut stream).await? {
        ValidateConfigAnswer::Success => Ok("answer".into()),
        ValidateConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        ValidateConfigAnswer::MissingGlobal => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a `global_config.toml` file to validate.",
        )),
        ValidateConfigAnswer::MissingLocal => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a `local_config.toml` file to validate.",
        )),
        ValidateConfigAnswer::MissingBoth => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod doesn't have a either a `local_config.toml` or a `global_config.toml` file to validate.",
        )),
        ValidateConfigAnswer::InvalidGlobal(error) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Global configuration validation failed:\n{error}"),
        )),
        ValidateConfigAnswer::InvalidLocal(error) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Local configuration validation failed:\n{error}"),
        )),
        ValidateConfigAnswer::InvalidBoth(local_error, global_error) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Local configuration validation failed:\n{local_error}\n\nGlobal configuration validation failed:\n{global_error}"),
        )),
    }
}
