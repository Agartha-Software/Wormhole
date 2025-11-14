use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::WriteConfigArgs,
    ipc::{
        answers::WriteConfigAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn write(args: WriteConfigArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args.group);

    send_command(Command::WriteConfg(pod, args.overwrite), &mut stream).await?;

    match recieve_answer::<WriteConfigAnswer>(&mut stream).await? {
        WriteConfigAnswer::Success => Ok("Configuration created successfully!".into()),
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
        WriteConfigAnswer::WriteFailed(err) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("Failed to write the configuration: {err}"),
        )),
        WriteConfigAnswer::CantOverwrite => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Couldn't write configuration files, at least one of them already exist...\n--overwrite to overwrite existing files"
            ),
        )),
    }
}
