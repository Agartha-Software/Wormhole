use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::NewArgs,
    ipc::{
        answers::NewAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, NewRequest},
    },
};

pub async fn new(args: NewArgs, mut stream: Stream) -> io::Result<()> {
    let mut mountpoint = match args.mountpoint {
        Some(mountpoint) => Ok(mountpoint),
        None => std::env::current_dir().map(|path| path.join(args.name.clone())),
    }?;

    mountpoint = std::fs::canonicalize(mountpoint)?;

    let NewArgs {
        name,
        port,
        url,
        hostname,
        listen_url,
        additional_hosts,
        ..
    } = args;

    let request = NewRequest {
        mountpoint,
        name: name.clone(),
        port,
        url,
        hostname,
        listen_url,
        additional_hosts,
    };
    send_command(Command::New(request), &mut stream).await?;

    match recieve_answer::<NewAnswer>(&mut stream).await? {
        NewAnswer::Success => {
            println!("Pod '{name}' created with success.");
            Ok(())
        }
        NewAnswer::AlreadyExist => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "Pod already exist, couldn't create.",
        )),
        NewAnswer::InvalidIp => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "Given port is already used.",
        )),
        NewAnswer::BindImpossible(e) => {
            eprintln!("Failed to bind the given pod.");
            Err(e.into())
        }

        NewAnswer::FailedToCreatePod(e) => Err(e.into()),
    }
}
