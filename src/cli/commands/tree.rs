use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::{
        connection::{recieve_answer, send_command},
        IdentifyPodArgs,
    },
    data::tree_hosts::TreeData,
    ipc::{
        answers::TreeAnswer,
        commands::{Command, PodId},
    },
};

pub async fn tree(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<TreeData> {
    let pod = PodId::from(args);

    send_command(Command::Tree(pod), &mut stream).await?;

    match recieve_answer::<TreeAnswer>(&mut stream).await? {
        TreeAnswer::Tree(data) => Ok(*data),
        TreeAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        TreeAnswer::PodTreeFailed(io) => Err(io.into()),
    }
}
