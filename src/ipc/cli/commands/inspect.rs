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

pub async fn inspect(args: IdentifyPodArgs, mut stream: Stream) -> Result<(), io::Error> {
    let id = PodId::from(args);

    send_command(Command::Inspect(id), &mut stream).await?;
    match recieve_answer::<InspectAnswer>(&mut stream).await? {
        InspectAnswer::Information(info) => {
            println!("Pod informations:");
            println!("   hostname:\t\t{}", info.hostname);
            println!("   name:\t\t{}", info.name);
            println!("   mount:\t\t{:#?}", info.mount);
            println!(
                "   url:\t\t\t{}",
                info.url.unwrap_or(String::from("Undefined"))
            );
            println!("   connected peers:\t{:#?}", info.connected_peers);
            Ok(())
        }
        InspectAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
    }
}
