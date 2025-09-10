use futures_util::sink::SinkExt;
use futures_util::TryStreamExt;
use tokio_tungstenite::tungstenite::{protocol::WebSocketConfig, Message};

use crate::{commands::cli_commands::Cli, error::CliResult};

pub async fn cli_messager(ip: &str, cli: Cli) -> CliResult<String> {
    let (mut ws_stream, _) = tokio_tungstenite::connect_async_with_config(
        format!("ws://{}", ip),
        Some(
            WebSocketConfig::default()
                .max_message_size(None)
                .max_frame_size(None),
        ),
        false,
    )
    .await?;
    log::info!("Service connected at ws://{ip}");

    let bytes = bincode::serialize(&cli)?;
    ws_stream.send(Message::Binary(bytes.into())).await?;

    let mut has_error = false;
    let mut output = String::new();
    while let Ok(Some(msg)) = ws_stream.try_next().await {
        if msg.is_text() {
            let response = msg.to_text()?;
            if response.contains("CliError") {
                has_error = true;
                log::error!("Service answer: {response}");
            } else {
                println!("{response}");
            }
            output.push_str(&format!("{response}\n"));
        }
    }

    log::info!("Connection closed");
    if has_error {
        Err(crate::error::CliError::Message { reason: output })
    } else {
        Ok(output)
    }
}
