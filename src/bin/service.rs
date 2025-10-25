// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use std::collections::HashMap;
/**DOC
 * Important variables to know :
 * nfa_rx - nfa_tx
 *  Use nfa_tx to send a file related message to the newtork_file_actions function
 *
 * Important functions to know :
 *
 * local_cli_watchdog
 *  this is the handle linked to the terminal, that will terminate the
 *  program if CTRL-D
 *
 * newtork_file_actions
 *  reads a message (supposely emitted by a peer) related to files actions
 *  and execute instructions on the disk
 */
use std::env;
use std::io::IsTerminal;
use std::process::ExitCode;

use clap::builder::Str;
use tokio::sync::mpsc::{self, UnboundedSender};

use wormhole::ipc::service::start_commands_listeners;
use wormhole::pods::pod::Pod;

#[cfg(target_os = "windows")]
use winfsp::winfsp_init;
use wormhole::signals::handle_signals;

const DEFAULT_ADDRESS: &str = "127.0.0.1:8081";

#[tokio::main]
async fn main() -> ExitCode {
    let (interrupt_tx, interrupt_rx) = mpsc::unbounded_channel::<()>();
    let (signals_tx, signals_rx) = mpsc::unbounded_channel::<()>();

    let mut pods: HashMap<String, Pod> = HashMap::new();

    if env::args().any(|arg| arg == "-h" || arg == "--help") {
        println!("Usage: wormholed <IP>\n\nIP is the node address, default at {DEFAULT_ADDRESS}");
        return ExitCode::SUCCESS;
    }

    env_logger::init();

    #[cfg(target_os = "windows")]
    match winfsp_init() {
        Ok(_token) => log::trace!("Obtained fsp token!"),
        Err(err) => {
            eprintln!("WindowsFSP failed to start, verify your installation: {err}");
            return ExitCode::FAILURE;
        }
    }

    let ip_string = env::args().filter(|arg| arg != "--nodeamon").nth(1);
    let terminal_handle =
        if std::io::stdout().is_terminal() || env::args().any(|arg| arg == "--nodeamon") {
            Some(tokio::spawn(terminal_watchdog(interrupt_tx)))
        } else {
            println!("Starting in deamon mode");
            None
        };
    let signals_task = tokio::spawn(handle_signals(signals_tx, interrupt_rx));
    log::trace!("Starting service on {:?}", ip_string);

    if let Err(err) = start_commands_listeners(&mut pods, ip_string, signals_rx).await {
        log::error!("Listener Error: {err}");
    }

    if let Some(terminal_handle) = terminal_handle {
        terminal_handle.abort();
    }

    signals_task
        .await
        .unwrap_or_else(|e| panic!("Signals handler failed to join: {e}"));

    log::info!("Stopping");
    stop_all_pods(pods).await
}

async fn stop_all_pods(pods: HashMap<String, Pod>) -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    for (name, pod) in pods.into_iter() {
        match pod.stop().await {
            Ok(()) => log::info!("Stopped pod '{name}'"),
            Err(e) => {
                eprintln!("Pod '{name}' failed be stopped: {e}");
                status = ExitCode::FAILURE
            }
        }
    }
    log::info!("Stopped");
    status
}

// NOTE - old watchdog brought here for debug purposes
pub async fn terminal_watchdog(tx: UnboundedSender<()>) {
    let mut stdin = tokio::io::stdin();
    let mut buf = vec![0; 1024];

    while let Ok(read) = tokio::io::AsyncReadExt::read(&mut stdin, &mut buf).await {
        // NOTE -  on ctrl-D -> quit
        match read {
            0 => {
                let _ = tx.send(());
                return;
            }
            _ => (),
        };
    }
}
