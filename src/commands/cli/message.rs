use futures_util::sink::SinkExt;
use futures_util::TryStreamExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{commands::cli_commands::Cli, error::CliResult};

pub async fn cli_messager(ip: &str, cli: Cli) -> CliResult<String> {
    let (mut ws_stream, _) = connect_async(format!("ws://{}", ip)).await?;
    log::info!("Service connected at ws://{ip}");

    let bytes = bincode::serialize(&cli)?;
    ws_stream.send(Message::Binary(bytes)).await?;

    let mut has_error = false;
    let mut output = String::new();
    while let Ok(Some(msg)) = ws_stream.try_next().await {
        if msg.is_text() {
            let response = msg.to_text()?;
            println!("{response}");
            break;
        }
    }

    ws_stream.close(None).await?;
    log::info!("Connection closed");
    if has_error {
        Err(crate::error::CliError::Message { reason: output })
    } else {
        Ok(output)
    }
}
