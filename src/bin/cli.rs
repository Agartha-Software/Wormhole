// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use clap::Parser;
use std::process::ExitCode;
use wormhole::cli::Cli;
use wormhole::ipc::cli::{command_network, start_local_socket};
use wormhole::ipc::service::SOCKET_DEFAULT_NAME;

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init();
    log::trace!("Starting cli!");
    let cmd = Cli::parse();
    log::debug!("Command found: {cmd:?}");

    let stream = match start_local_socket(&cmd.socket).await {
        //TODO: don't open stream on local cmd
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Connection to the service failed: {}: {err}", err.kind());
            if cmd.socket.as_str() == SOCKET_DEFAULT_NAME {
                eprintln!("Check if the service is running.");
            } else {
                eprintln!(
                    "Check if a service listening to '{}' is running.",
                    cmd.socket
                );
            }
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
