use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        answers::TreeAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn tree(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<()> {
    let pod = PodId::from(args);

    send_command(Command::Tree(pod), &mut stream).await?;

    match recieve_answer::<TreeAnswer>(&mut stream).await? {
        TreeAnswer::Tree(tree) => {
            println!("{}", tree);
            Ok(())
        }
        TreeAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        TreeAnswer::PodTreeFailed(io) => Err(io.into()),
    }
}
