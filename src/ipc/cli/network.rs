use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::Cli,
    ipc::cli::commands::{freeze, gethosts, new, unfreeze},
};

pub async fn command_network(cmd: Cli, stream: Stream) -> Result<(), std::io::Error> {
    match cmd {
        Cli::New(args) => new(args, stream).await,
        Cli::Freeze(args) => freeze(args, stream).await,
        Cli::UnFreeze(args) => unfreeze(args, stream).await,
        Cli::Template(_args) => todo!(),
        Cli::Inspect(_args) => todo!(),
        Cli::GetHosts(args) => gethosts(args, stream).await,
        Cli::Tree(_args) => todo!(),
        Cli::Remove(_args) => todo!(),
        Cli::Apply(_args) => todo!(),
        Cli::Restore(_args) => todo!(),
        Cli::Status => todo!(),
        Cli::Start => todo!(),
        Cli::Stop => todo!(),
    }
}
