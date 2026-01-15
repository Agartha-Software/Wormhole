use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::commands::{
        check, generate, gethosts, inspect, list_pods, new, redundancy_status, remove, show,
        status, tree,
    },
    cli::{CliCommand, ConfigCommand},
};

type Answer = String;

pub async fn command_network(cmd: CliCommand, stream: Stream) -> Result<Answer, std::io::Error> {
    match cmd {
        CliCommand::New(args) => new(args, stream).await,
        CliCommand::Inspect(args) => inspect(args, stream).await,
        CliCommand::GetHosts(args) => gethosts(args, stream).await,
        CliCommand::Tree(args) => tree(args, stream).await,
        CliCommand::Remove(args) => remove(args, stream).await,
        CliCommand::Config(ConfigCommand::Generate(args)) => generate(args, stream).await,
        CliCommand::Config(ConfigCommand::Show(args)) => show(args, stream).await,
        CliCommand::Config(ConfigCommand::Check(args)) => check(args, stream).await,
        CliCommand::Status => status(stream).await,
        CliCommand::ListPods => list_pods(stream).await,
        CliCommand::RedundancyStatus(args) => redundancy_status(args, stream).await,
    }
}
