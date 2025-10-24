// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use clap::{Arg, Command, Parser, Subcommand};
use interprocess::local_socket::traits::tokio::Stream;
use std::env;
use std::process::ExitCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wormhole::{
    commands::{cli::start, cli_commands::Cli},
    error::{CliError, CliResult, CliSuccess},
    ipc::{cli::start_local_socket, CommandAnswer},
};

#[tokio::main]
async fn main() -> ExitCode {
    // let matches = Command::new("wormhole")
    //     .version("1.0")
    //     .author("Kevin K. <kbknapp@gmail.com>")
    //     .about("Does awesome things")
    //     .arg(
    //         Arg::new("CONFIG")
    //             .short('c')
    //             .long("config")
    //             .help("Sets a custom config file"),
    //     )
    //     .subcommand(
    //         Command::new("test")
    //             .about("controls testing features")
    //             .version("1.3")
    //             .author("Someone E. <someone_else@other.com>")
    //             .arg(
    //                 Arg::new("verbose")
    //                     .short('v')
    //                     .help("print test information verbosely"),
    //             ),
    //     )
    //     .get_matches();
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

    let (mut read, mut write) = stream.split();
    let mut recived_answer = Vec::new();

    let serialized =
        bincode::serialize(&cmd).expect("Can't serialize cli command, shouldn't be possible .");

    write.write_all(&serialized).await.unwrap();
    let _recv = read.read_buf(&mut recived_answer).await.unwrap();

    let out = match cmd {
        Cli::Start(identify_pod_args) => start(identify_pod_args),
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
    };

    let answer = match bincode::deserialize::<Cli>(&recived_answer) {
        Ok(answer) => answer,
        Err(err) => {
            eprintln!("Command recieved isn't recognized: {err}");
            return ExitCode::FAILURE;
        }
    };

    return ExitCode::SUCCESS;
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
