use std::io;

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::NewArgs,
    ipc::{
        cli::connection::{recieve_answer, send_command},
        commands::{Command, NewAnswer, NewRequest},
    },
};

pub async fn new(args: NewArgs, mut stream: Stream) -> Result<(), io::Error> {
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
        NewAnswer::Success => println!("Pod '{name}' created with success."),
        NewAnswer::AlreadyExist => eprintln!("Pod '{name}' already exist, couldn't create."),
        NewAnswer::InvalidIp => eprintln!("Given port is already used."),
        NewAnswer::BindImpossible(e) => eprintln!("Service failed to bind the pod: {e}"),
        NewAnswer::FailedToCreatePod(e) => eprintln!("Service failed to create the pod: {e}"),
    }
    Ok(())
}
