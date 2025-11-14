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
        ValidateConfigAnswer::Success(answer) => Ok(answer),
        ValidateConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
    }
}
