use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::{
        connection::{recieve_answer, send_command},
        print_err, IdentifyPodArgs,
    },
    ipc::{
        answers::UnfreezeAnswer,
        commands::{Command, PodId},
    },
};

pub async fn unfreeze(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<String> {
    let id = PodId::from(args);

    send_command(Command::Freeze(id), &mut stream).await?;
    match recieve_answer::<UnfreezeAnswer>(&mut stream).await? {
        UnfreezeAnswer::Success(name) => Ok(format!("Pod '{name}' unfrozen successfully!")),
        UnfreezeAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        UnfreezeAnswer::AlreadyUnfrozen => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "The given pod is already unfrozen.",
        )),
        UnfreezeAnswer::CouldntBind(e) => {
            print_err("Failed to bind the given pod:");
            Err(e.into())
        }
        UnfreezeAnswer::PodCreationFailed(e) => {
            print_err("Failed to unfreeze the given pod:");
            Err(e.into())
        }
    }
}
