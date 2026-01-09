use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::connection::{recieve_answer, send_command},
    cli::IdentifyPodAndConfigArgs,
    ipc::{
        answers::ShowConfigAnswer,
        commands::{Command, PodId},
    },
};

pub async fn show(args: IdentifyPodAndConfigArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args.group);

    send_command(Command::ShowConfig(pod, args.config_type), &mut stream).await?;

    match recieve_answer::<ShowConfigAnswer>(&mut stream).await? {
        ShowConfigAnswer::SuccessBoth(mut local_str, mut global_str) => {
            local_str = local_str.replace("\n", "\n   ");
            global_str = global_str.replace("\n", "\n   ");
            Ok(format!(
                "Local configuration:\n   {local_str}\nGlobal configuration:\n   {global_str}"
            ))
        }
        ShowConfigAnswer::SuccessLocal(local_str) => Ok(local_str),
        ShowConfigAnswer::SuccessGlobal(global_str) => Ok(global_str),
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
