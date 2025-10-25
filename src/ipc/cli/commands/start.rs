use std::io;

use interprocess::local_socket::tokio::Stream;
use serde::{Deserialize, Serialize};

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        cli::connection::{recieve_answer, send_command},
        commands::{Command, StartRequest},
    },
};

#[derive(Debug, Serialize, Deserialize)]
pub enum StartAnswer {
    Success,
}

pub async fn start(args: IdentifyPodArgs, mut stream: Stream) -> Result<(), io::Error> {
    let id = if let Some(name) = args.group.name {
        StartRequest::Name(name)
    } else {
        if let Some(path) = args.group.path {
            StartRequest::Path(path)
        } else {
            panic!("One of path or name should always be defined, if both are missing Clap should block the cmd")
        }
    };

    send_command(Command::Start(id), &mut stream).await?;
    match recieve_answer::<StartAnswer>(&mut stream).await? {
        StartAnswer::Success => {
            println!("Start is not yet implemented! You need to manually restart the service by hand... This feature is coming soon!");
            Ok(())
        }
    }
}
