use std::io;

use crate::{cli::GetHostsArgs, ipc::answers::GetHostsAnswer, ipc::commands::GetHostsRequest};
use interprocess::local_socket::tokio::Stream;

use crate::ipc::{
    cli::connection::{recieve_answer, send_command},
    commands::Command,
};

pub async fn gethosts(args: GetHostsArgs, mut stream: Stream) -> Result<(), io::Error> {
    send_command(
        Command::GetHosts(GetHostsRequest { path: args.path }),
        &mut stream,
    )
    .await?;
    match recieve_answer::<GetHostsAnswer>(&mut stream).await? {
        GetHostsAnswer::Hosts(hosts) => {
            println!("Hosts: {:?}", hosts);
            Ok(())
        }
        GetHostsAnswer::FileNotInsideARunningPod => Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "The given path does not descend a pod",
        )),
        GetHostsAnswer::FileNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given path couldn't be found inside the pod",
        )),
        GetHostsAnswer::WrongFileType(_) => Err(io::Error::new(
            io::ErrorKind::IsADirectory,
            "Only files have hosts.",
        )),
        GetHostsAnswer::FailedToGetHosts(io_error) => Err(io_error.into()),
    }
}
