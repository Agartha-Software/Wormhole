use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::Cli,
    ipc::cli::commands::{new, unfreeze},
};

pub async fn command_network(cmd: Cli, stream: Stream) -> Result<(), std::io::Error> {
    match cmd {
        Cli::New(args) => todo!(),
        Cli::Freeze(args) => todo!(),
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

// let status = match Cli::parse_from(cli_args) {
//     Cli::Start(args) => commands::cli::start(ip, args),
//     Cli::Stop(args) => commands::cli::stop(ip, args),
//     Cli::Template(args) => {
//         log::info!("creating network {:?}", args.name.clone());
//         commands::cli::templates(&args.mountpoint.unwrap_or(".".into()), &args.name)
//     }
//     Cli::New(args) => {
//         log::info!("creating pod");
//         commands::cli::new(ip, args)
//     }
//     Cli::Remove(args) => {
//         log::info!("removing pod");
//         commands::cli::remove(ip, args)
//     }
//     Cli::Inspect => {
//         log::warn!("inspecting pod");
//         todo!("inspect");
//     }
//     Cli::GetHosts(args) => commands::cli::get_hosts(ip, args),
//     Cli::Tree(args) => commands::cli::tree(ip, args),
//     Cli::Apply(args) => {
//         log::warn!("reloading pod");
//         commands::cli::apply(ip, args)
//     }
//     Cli::Status => commands::cli::status(ip),
//     Cli::Restore(args) => {
//         log::info!("retore a specific file config");
//         commands::cli::restore(ip, args)
//     }
//     Cli::Interrupt => {
//         log::warn!("interrupt command");
//         todo!("interrupt");
//     }
// };
// if let Err(e) = &status {
//     log::error!("CLI: error reported: {e}");
// } else {
//     log::info!("CLI: no error reported")
// };
// status.map(|s| {
//     println!("{s}");
//     ()
// })
