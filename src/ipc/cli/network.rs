use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::{CliCommand, ConfigCommand},
    ipc::cli::commands::{gethosts, inspect, new, remove, status, tree, write},
};

type Answer = String;

pub async fn command_network(cmd: CliCommand, stream: Stream) -> Result<Answer, std::io::Error> {
    match cmd {
        CliCommand::New(args) => new(args, stream).await,
        CliCommand::Inspect(args) => inspect(args, stream).await,
        CliCommand::GetHosts(args) => gethosts(args, stream).await,
        CliCommand::Tree(args) => tree(args, stream).await,
        CliCommand::Remove(args) => remove(args, stream).await,
        CliCommand::Config(ConfigCommand::Write(args)) => write(args, stream).await,
        CliCommand::Config(ConfigCommand::Show) => todo!(),
        CliCommand::Config(ConfigCommand::Validate) => todo!(),
        CliCommand::Status => status(stream).await,
    }
}
