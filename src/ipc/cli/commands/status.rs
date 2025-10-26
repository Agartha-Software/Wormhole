use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::ipc::{
    answers::StatusAnswer,
    cli::connection::{recieve_answer, send_command},
    commands::Command,
};

pub async fn status(mut stream: Stream) -> io::Result<()> {
    send_command(Command::Status, &mut stream).await?;

    match recieve_answer::<StatusAnswer>(&mut stream).await? {
        StatusAnswer::Success => {
            println!("Service existing");
            Ok(())
        }
    }
}
