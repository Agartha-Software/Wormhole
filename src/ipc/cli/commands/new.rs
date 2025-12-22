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

pub async fn new(args: NewArgs, mut stream: Stream) -> io::Result<String> {
    let mountpoint = match args.mountpoint {
        Some(mountpoint) => Ok(mountpoint),
        None => std::env::current_dir().map(|path| path.join(args.name.clone())),
    }?;

    let NewArgs {
        name,
        url,
        hostname,
        additional_hosts,
        public_url,
        ip_address,
        port,
        ..
    } = args;

    let request = NewRequest {
        mountpoint: mountpoint.clone(),
        name: name.clone(),
        ip_address,
        port,
        public_url,
        url,
        hostname,
        additional_hosts,
    };
    send_command(Command::New(request), &mut stream).await?;

    match recieve_answer::<NewAnswer>(&mut stream).await? {
        NewAnswer::Success(listen_url) => Ok(format!(
            "Pod '{name}' created with success, listening to '{listen_url}'."
        )),
        NewAnswer::AlreadyExist => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Pod with name '{name}' already exist, couldn't create."),
        )),
        NewAnswer::AlreadyMounted => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("A pod at {mountpoint:#?} is already mounted, couldn't create."),
        )),
        NewAnswer::InvalidIp => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "Given port is already used, couldn't create.",
        )),
        NewAnswer::BindImpossible(e) => {
            eprintln!("Failed to bind the given pod:");
            Err(e.into())
        }
        NewAnswer::FailedToCreatePod(e) => {
            eprintln!("Failed to create the given pod:");
            Err(e.into())
        }
        NewAnswer::InvalidUrlIp => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The given Url to connect to is invalid.",
        )),
        NewAnswer::ConflictWithConfig(field) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("The field '{field}' have a conflicting value between args and configuration files."),
        )),
    }
}
