use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::cli::connection::{recieve_answer, send_command};
use crate::ipc::{answers::StatusAnswer, commands::Command};

pub async fn status(mut stream: Stream) -> io::Result<String> {
    send_command(Command::Status, &mut stream).await?;

    match recieve_answer::<StatusAnswer>(&mut stream).await? {
        StatusAnswer::Success => Ok(String::from("Service online.")),
    }
}
