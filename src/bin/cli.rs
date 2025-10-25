// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use clap::Parser;
use std::process::ExitCode;
use wormhole::cli::Cli;
use wormhole::ipc::cli::{command_network, start_local_socket};

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init();
    log::trace!("Starting cli!");
    let cmd = Cli::parse();
    log::debug!("Command found: {cmd:?}");

    let stream = match start_local_socket().await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Connection to the service failed: {}: {err}", err.kind());
            return ExitCode::FAILURE;
        }
    };

    match command_network(cmd, stream).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Command recieved isn't recognized: {err}");
            return ExitCode::FAILURE;
        }
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
