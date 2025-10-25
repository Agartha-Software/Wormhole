use std::io;

use crate::ipc::commands::UnfreezeAnswer;
use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

pub async fn unfreeze(args: IdentifyPodArgs, mut stream: Stream) -> Result<(), io::Error> {
    let id = if let Some(name) = args.group.name {
        PodId::Name(name)
    } else {
        if let Some(path) = args.group.path {
            PodId::Path(path)
        } else {
            panic!("One of path or name should always be defined, if both are missing Clap should block the cmd")
        }
    };

    send_command(Command::Unfreeze(id), &mut stream).await?;
    match recieve_answer::<UnfreezeAnswer>(&mut stream).await? {
        UnfreezeAnswer::Success => {
            println!("Start is not yet implemented! You need to manually restart the service by hand... This feature is coming soon!");
            Ok(())
        }
    }
}
