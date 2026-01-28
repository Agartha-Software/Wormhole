use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::cli::connection::{recieve_answer, send_command};
use crate::ipc::{answers::ListPodsAnswer, commands::Command};

pub async fn list_pods(mut stream: Stream) -> io::Result<String> {
    send_command(Command::ListPods, &mut stream).await?;

    match recieve_answer::<ListPodsAnswer>(&mut stream).await? {
        ListPodsAnswer::Pods(pods) => Ok(if pods.is_empty() {
            pods.join("\n")
        } else {
            "No pods for now.".to_owned()
        }),
    }
}
