use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::Cli,
    ipc::cli::commands::{freeze, new, unfreeze},
};

pub async fn command_network(cmd: Cli, stream: Stream) -> Result<(), std::io::Error> {
    match cmd {
        Cli::New(args) => new(args, stream).await,
        Cli::Freeze(args) => freeze(args, stream).await,
        Cli::UnFreeze(args) => unfreeze(args, stream).await,
        Cli::Template(args) => todo!(),
        Cli::Inspect(args) => todo!(),
        Cli::GetHosts(args) => todo!(),
        Cli::Tree(args) => todo!(),
        Cli::Remove(args) => todo!(),
        Cli::Apply(args) => todo!(),
        Cli::Restore(args) => todo!(),
        Cli::Status => todo!(),
        Cli::Start => todo!(),
        Cli::Stop => todo!(),
    }
}
