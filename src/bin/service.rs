// In rust we code
// In code we trust
// AgarthaSoftware - 2024

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
use clap::Parser;
use std::io::IsTerminal;
use std::process::ExitCode;
use tokio::sync::mpsc::{self, UnboundedSender};
use wormhole::logging::custom_format;
use wormhole::service::clap::ServiceArgs;
use wormhole::service::Service;

#[cfg(target_os = "windows")]
use winfsp::winfsp_init;
use wormhole::signals::handle_signals;

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::Builder::from_default_env()
        .format(custom_format)
        .init();

    let (interrupt_tx, interrupt_rx) = mpsc::unbounded_channel::<()>();
    let (signals_tx, signals_rx) = mpsc::unbounded_channel::<()>();
    let args = ServiceArgs::parse();

    #[cfg(target_os = "windows")]
    match winfsp_init() {
        Ok(_token) => log::trace!("Obtained fsp token!"),
        Err(err) => {
            eprintln!("WindowsFSP failed to start, verify your installation: {err}");
            return ExitCode::FAILURE;
        }
    }

    let terminal_handle = if std::io::stdout().is_terminal() || args.nodeamon {
        Some(tokio::spawn(terminal_watchdog(interrupt_tx)))
    } else {
        println!("Starting in deamon mode");
        None
    };
    let signals_task = tokio::spawn(handle_signals(signals_tx, interrupt_rx));

    let Some(mut service) = Service::new(args).await else {
        return ExitCode::FAILURE;
    };

    if let Err(err) = service.start_commands_listeners(signals_rx).await {
        eprintln!("{err}");
    }

    if let Some(terminal_handle) = terminal_handle {
        terminal_handle.abort();
    }

    signals_task
        .await
        .unwrap_or_else(|e| panic!("Signals handler failed to join: {e}"));

    log::info!("Stopping");
    service.stop_all_pods().await
}

pub async fn terminal_watchdog(tx: UnboundedSender<()>) {
    let mut stdin = tokio::io::stdin();
    let mut buf = vec![0; 1024];

    while let Ok(read) = tokio::io::AsyncReadExt::read(&mut stdin, &mut buf).await {
        // Quit on ctrl-D
        if read == 0 {
            let _ = tx.send(());
            return;
        };
    }
}
