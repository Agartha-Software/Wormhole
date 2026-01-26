use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::connection::{recieve_answer, send_command},
    cli::{Mode, RemoveArgs},
    ipc::answers::RemoveAnswer,
    ipc::commands::{Command, PodId, RemoveRequest},
};

pub async fn remove(args: RemoveArgs, mut stream: Stream) -> Result<String, io::Error> {
    let pod = PodId::from(args.group);

    send_command(
        Command::Remove(RemoveRequest {
            pod,
            mode: Mode::Simple, //args.mode
        }),
        &mut stream,
    )
    .await?;
    match recieve_answer::<RemoveAnswer>(&mut stream).await? {
        RemoveAnswer::Success => Ok(String::from("Pod successfully removed.")),
        RemoveAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        RemoveAnswer::PodStopFailed(err) => Err(io::Error::new(
            err.kind,
            format!("The pod couldn't be removed cleanly:\n{}", err.error),
        )),
    }
}
