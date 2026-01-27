use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::cli::connection::{recieve_answer, send_command};
use crate::ipc::answers::StatusSuccess;
use crate::ipc::{answers::StatusAnswer, commands::Command};

pub async fn status(mut stream: Stream) -> io::Result<String> {
    send_command(Command::Status, &mut stream).await?;

    match recieve_answer::<StatusAnswer>(&mut stream).await? {
        StatusAnswer::Success(StatusSuccess { nickname, running, frozen }) => Ok(format!(
            "Service running: \n\
            \tNickname:\t{}\n\
            \tRunning Pods:\t[ {} ]\n\
            \tFrozen Pods:\t[ {} ]",
            nickname.escape_debug(),
            running.join(", "),
            frozen.join(", ")
        )),
    }
}
