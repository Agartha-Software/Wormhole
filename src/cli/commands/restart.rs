use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::{
        connection::{recieve_answer, send_command},
        print_err, IdentifyPodArgs,
    },
    ipc::{
        answers::RestartAnswer,
        commands::{Command, PodId},
    },
};

pub async fn restart(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<String> {
    let id = PodId::from(args);

    send_command(Command::Restart(id), &mut stream).await?;
    match recieve_answer::<RestartAnswer>(&mut stream).await? {
        RestartAnswer::Success(name) => Ok(format!("Pod '{name}' restarted successfully!")),
        RestartAnswer::PodFrozen => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The pod is currently frozen, use unfreeze instead to start the pod.",
        )),
        RestartAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        RestartAnswer::PodBlock => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Failed to restart pod, please retry.",
        )),
        RestartAnswer::PodStopFailed(err) => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("An error occured while stopping the pod: {err}"),
        )),
        RestartAnswer::CouldntBind(e) => {
            print_err("Failed to bind the restarting pod:");
            Err(e.into())
        }
        RestartAnswer::PodCreationFailed(e) => {
            print_err("Failed to recreate the restarting pod:");
            Err(e.into())
        }
    }
}
