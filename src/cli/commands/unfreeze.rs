use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::connection::{recieve_answer, send_command},
    cli::IdentifyPodArgs,
    ipc::answers::UnfreezeAnswer,
    ipc::commands::{Command, PodId},
};

pub async fn _unfreeze(args: IdentifyPodArgs, mut stream: Stream) -> Result<(), io::Error> {
    let id = PodId::from(args);

    send_command(Command::Unfreeze(id), &mut stream).await?;
    match recieve_answer::<UnfreezeAnswer>(&mut stream).await? {
        UnfreezeAnswer::Success => {
            println!("Unfreeze is not yet implemented! You need to manually restart the service by hand... This feature is coming soon!");
            Ok(())
        }
    }
}
