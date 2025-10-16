use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};


pub async fn handle_signals(tx: UnboundedSender<()>, interrupt_rx: UnboundedReceiver<()>) {
    #[cfg(unix)]
    {
        handle_signals_unix(tx, interrupt_rx).await;
    }

    #[cfg(windows)]
    {
        handle_signals_windows(tx, interrupt_rx).await;
    }
}

#[cfg(unix)]
pub async fn handle_signals_unix(tx: UnboundedSender<()>, mut interrupt_rx: UnboundedReceiver<()>) {
    use tokio::signal::unix;

    let mut sigint = unix::signal(unix::SignalKind::interrupt()).expect("failed to bind SIGINT");
    let mut sigterm = unix::signal(unix::SignalKind::terminate()).expect("failed to bind SIGTERM");

    log::info!("Unix signal handler initialised, waiting for SIGINT or SIGTERM…");

    tokio::select! {
        _ = sigint.recv() => {
            log::info!("Quiting by Signal: SIGINT");
            let _ = tx.send(());
        }
        _ = sigterm.recv() => {
            log::info!("Quiting by Signal: SIGTERM");
            let _ = tx.send(());
        }
        _ = interrupt_rx.recv() => {
            log::info!("Quiting by Ctrl+D! (EOF)");
            let _ = tx.send(());
        }
    }
}

#[cfg(windows)]
pub async fn handle_signals_windows(tx: UnboundedSender<()>, mut interrupt_rx: UnboundedReceiver<()>) {
    log::info!("Windows signal handler initialised…");

    let mut sig_c = tokio::signal::windows::ctrl_c().expect("Failed to register ctrl_c");
    let mut sig_break =
        tokio::signal::windows::ctrl_break().expect("Failed to register ctrl_break");

    tokio::select! {
        _ = sig_c.recv() => {
            log::info!("Quiting by Signal: CTRL+C");
            let _ = tx.send(());
        }
        _ = sig_break.recv() => {
            log::info!("Quiting by Signal: CTRL+BREAK");
            let _ = tx.send(());
        }
        _ = interrupt_rx.recv() => {
            log::info!("Quiting by Ctrl-Z (EOF)");
            let _ = tx.send(());
        }
    }
}
