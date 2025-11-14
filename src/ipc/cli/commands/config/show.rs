use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        answers::ShowConfigAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn show(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args);

    send_command(Command::ShowConfig(pod), &mut stream).await?;

    match recieve_answer::<ShowConfigAnswer>(&mut stream).await? {
        ShowConfigAnswer::Success(mut local_str, mut global_str) => {
            local_str = local_str.replace("\n", "\n   ");
            global_str = global_str.replace("\n", "\n   ");
            Ok(format!(
                "Local configuration:\n   {local_str}\nGlobal configuration:\n   {global_str}"
            ))
        }
        ShowConfigAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        ShowConfigAnswer::ConfigBlock => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Failed to access the pod configuration.",
        )),
    }
}
