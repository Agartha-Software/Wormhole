use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::connection::{recieve_answer, send_command},
    cli::IdentifyPodArgs,
    ipc::answers::FreezeAnswer,
    ipc::commands::{Command, PodId},
};

pub async fn freeze(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<String> {
    let id = PodId::from(args);

    send_command(Command::Freeze(id), &mut stream).await?;
    match recieve_answer::<FreezeAnswer>(&mut stream).await? {
        FreezeAnswer::Success(name) => Ok(format!("Pod '{name}' frozen successfully!")),
        FreezeAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        FreezeAnswer::AlreadyFrozen => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "The given pod is already frozen.",
        )),
        FreezeAnswer::PodBlock => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Failed to freeze pod, please retry.",
        )),
        FreezeAnswer::PodStopFailed(err) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("The pod has been frozen but couldn't be stopped cleanly: {err}"),
        )),
    }
}
