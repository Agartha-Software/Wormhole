use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::CliCommand,
    ipc::cli::commands::{freeze, gethosts, inspect, new, status, unfreeze},
};

pub async fn command_network(cmd: CliCommand, stream: Stream) -> Result<(), std::io::Error> {
    match cmd {
        CliCommand::New(args) => new(args, stream).await,
        CliCommand::Freeze(args) => freeze(args, stream).await,
        CliCommand::UnFreeze(args) => unfreeze(args, stream).await,
        CliCommand::Template(_args) => todo!(),
        CliCommand::Inspect(args) => inspect(args, stream).await,
        CliCommand::GetHosts(args) => gethosts(args, stream).await,
        CliCommand::Tree(_args) => todo!(),
        CliCommand::Remove(_args) => todo!(),
        CliCommand::Apply(_args) => todo!(),
        CliCommand::Restore(_args) => todo!(),
        CliCommand::Status => status(stream).await,
        CliCommand::Start => todo!(),
        CliCommand::Stop => todo!(),
    }
}
