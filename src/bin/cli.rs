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

    let stream = match start_local_socket(cmd.socket).await {
        //TODO: don't open stream on local cmd
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Connection to the service failed: {}: {err}", err.kind());
            eprintln!("Check if the service is running.");
            return ExitCode::FAILURE;
        }
    };
    log::trace!("Connection with the service open.");

    match command_network(cmd.command, stream).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    }
}
