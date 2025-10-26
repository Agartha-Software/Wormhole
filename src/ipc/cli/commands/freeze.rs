use std::io;

use crate::ipc::answers::FreezeAnswer;
use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn freeze(args: IdentifyPodArgs, mut stream: Stream) -> Result<(), io::Error> {
    let id = PodId::from(args);

    send_command(Command::Unfreeze(id), &mut stream).await?;
    match recieve_answer::<FreezeAnswer>(&mut stream).await? {
        FreezeAnswer::Success => {
            println!("Start is not yet implemented! You need to manually restart the service by hand... This feature is coming soon!");
            Ok(())
        }
    }
}
