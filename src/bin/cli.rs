// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use clap::Parser;
use std::process::ExitCode;
use wormhole::cli::{command_network, print_err, start_local_socket, Cli};
use wormhole::service::socket::SOCKET_DEFAULT_NAME;

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
            print_err(format!(
                "Connection to the service failed: {}: {err}",
                err.kind()
            ));
            if cmd.socket.as_str() == SOCKET_DEFAULT_NAME {
                print_err("Check if the service is running.");
            } else {
                print_err(format!(
                    "Check if a service listening to '{}' is running.",
                    cmd.socket,
                ));
            }
            return ExitCode::FAILURE;
        }
    };
    log::trace!("Connection with the service open.");

    match command_network(cmd.command, stream).await {
        Ok(answer) => {
            println!("{}", answer);
            ExitCode::SUCCESS
        }
        Err(err) => {
            print_err(err.to_string());
            ExitCode::FAILURE
        }
    }
}
