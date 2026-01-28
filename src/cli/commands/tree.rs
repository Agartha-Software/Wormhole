use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::{
        connection::{recieve_answer, send_command},
        IdentifyPodArgs,
    },
    ipc::{
        answers::TreeAnswer,
        commands::{Command, PodId},
    },
};

pub async fn tree(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args);

    send_command(Command::Tree(pod), &mut stream).await?;

    match recieve_answer::<TreeAnswer>(&mut stream).await? {
        TreeAnswer::Tree(data) => Ok(format!("{:?}", *data)),
        TreeAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        TreeAnswer::PodTreeFailed(io) => Err(io.into()),
    }
}
