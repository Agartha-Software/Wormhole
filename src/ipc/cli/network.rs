use interprocess::local_socket::tokio::Stream;

use crate::{cli::Cli, ipc::cli::commands::start};

pub async fn command_network(cmd: Cli, stream: Stream) -> Result<(), std::io::Error> {
    match cmd {
        Cli::Start(identify_pod_args) => start(identify_pod_args, stream).await,
        Cli::Stop(identify_pod_args) => todo!(),
        Cli::Template(template_arg) => todo!(),
        Cli::New(pod_args) => todo!(),
        Cli::Inspect => todo!(),
        Cli::GetHosts(get_hosts_args) => todo!(),
        Cli::Tree(tree_args) => todo!(),
        Cli::Status => todo!(),
        Cli::Remove(remove_args) => todo!(),
        Cli::Apply(pod_conf) => todo!(),
        Cli::Restore(pod_conf) => todo!(),
        Cli::Interrupt => todo!(),
    }
}
