use clap::Parser;
use futures_util::sink::SinkExt;
use futures_util::TryStreamExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::commands::cli_commands::Cli;

// #[derive(Parser)]
// #[command(name = "wormhole-cli")]
// #[command(about = "CLI pour contrôler le service Wormhole", long_about = None)]
// pub struct CliMessage {
//     /// La commande à envoyer au service (ex. "join <network>", "status")
//     pub command: String,
// }

pub async fn cli_messager(ip: &str, cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Se connecter au service sur le port dédié pour la CLI (par exemple, 8081)
    let (mut ws_stream, _) = connect_async(format!("ws://{}", ip)).await?;
    println!("Connecté au service sur {}", format!("ws://{}", ip));

    // Envoyer la commande au service
    let bytes = bincode::serialize(&cli)?;
    ws_stream.send(Message::Binary(bytes)).await?;
    println!("Commande envoyée : {:?}", cli);

    // Attendre la réponse du service
    while let Ok(Some(msg)) = ws_stream.try_next().await {
        if msg.is_text() {
            let response = msg.to_text()?;
            println!("Réponse du service : {}", response);
            break;
        }
    }

    ws_stream.close(None).await?;
    println!("Connexion fermée");
    Ok(())
}
