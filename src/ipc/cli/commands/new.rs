use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::NewArgs,
    ipc::{
        cli::connection::{recieve_answer, send_command},
        commands::Command,
    },
};

pub async fn new(args: NewArgs, mut stream: Stream) -> Result<(), io::Error> {
    // send_command(Command::Start(id), &mut stream).await?;
    // match recieve_answer::<StartAnswer>(&mut stream).await? {
    //     StartAnswer::Success => {
    //         println!("Start is not yet implemented! You need to manually restart the service by hand... This feature is coming soon!");
    //     }
    // }
    Ok(())
}
