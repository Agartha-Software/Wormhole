use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::{
    cli::{
        connection::{recieve_answer, send_command},
        print_err, NewArgs,
    },
    ipc::{
        answers::NewAnswer,
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
        mut additional_hosts,
        allow_other_users,
        ip_address,
        port,
        ..
    } = args;

    if let Some(url) = url {
        additional_hosts.insert(0, url);
    }

    let request = NewRequest {
        mountpoint: mountpoint.clone(),
        name: name.clone(),
        ip_address,
        port,
        hosts: additional_hosts,
        allow_other_users,
    };
    send_command(Command::New(request), &mut stream).await?;

    match recieve_answer::<NewAnswer>(&mut stream).await? {
        NewAnswer::Success(_, true) => Ok(format!(
            "Pod '{name}' successfully created, trying to connect..."
        )),
        NewAnswer::Success(listen_url, false) => Ok(format!(
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
        NewAnswer::PortAlreadyTaken => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "Given port already used.",
        )),
        NewAnswer::NoFreePortInRage => Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "No free port in range 40000-40100",
        )),
        NewAnswer::InvalidIp(e) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid Ip given, couldn't create: {e}"),
        )),
        NewAnswer::FailedToCreatePod(e) => {
            print_err("Failed to create the given pod:");
            Err(e.into())
        }
        NewAnswer::ConflictWithConfig(field) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("The field '{field}' have a conflicting value between args and configuration files."),
        )),
    }
}
