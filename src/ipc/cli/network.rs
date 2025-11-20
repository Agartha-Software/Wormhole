use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::{CliCommand, ConfigCommand},
    ipc::cli::commands::{check, gethosts, inspect, new, remove, save, show, status, tree},
};

type Answer = String;

pub async fn command_network(cmd: CliCommand, stream: Stream) -> Result<Answer, std::io::Error> {
    match cmd {
        CliCommand::New(args) => new(args, stream).await,
        CliCommand::Inspect(args) => inspect(args, stream).await,
        CliCommand::GetHosts(args) => gethosts(args, stream).await,
        CliCommand::Tree(args) => tree(args, stream).await,
        CliCommand::Remove(args) => remove(args, stream).await,
        CliCommand::Config(ConfigCommand::Save(args)) => save(args, stream).await,
        CliCommand::Config(ConfigCommand::Show(args)) => show(args, stream).await,
        CliCommand::Config(ConfigCommand::Check(args)) => check(args, stream).await,
        CliCommand::Status => status(stream).await,
    }
}
