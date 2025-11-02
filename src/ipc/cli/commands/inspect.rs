use std::io;

use crate::ipc::answers::InspectAnswer;
use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn inspect(args: IdentifyPodArgs, mut stream: Stream) -> Result<String, io::Error> {
    let id = PodId::from(args);

    send_command(Command::Inspect(id), &mut stream).await?;
    match recieve_answer::<InspectAnswer>(&mut stream).await? {
        InspectAnswer::Information(info) => Ok(format!(
            "Pod informations:\n\
   hostname:\t\t{}\n\
   name:\t\t{}\n\
   mount:\t\t{:#?}\n\
   url:\t\t\t{}\n\
   connected peers:\t{:#?}",
            info.hostname,
            info.name,
            info.mount,
            info.url.unwrap_or(String::from("Undefined")),
            info.connected_peers
        )),
        InspectAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
    }
}
