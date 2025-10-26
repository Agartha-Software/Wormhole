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
            println!("Informations:");
            println!("\thostname:\t{}", info.hostname);
            println!("\tname:\t{}", info.name);
            println!(
                "\turl:\t{}",
                info.url.unwrap_or(String::from("Not Defined"))
            );
            println!("\tconnected peers:\t{:?}", info.connected_peers);
            Ok(())
        }
        InspectAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
    }
}
