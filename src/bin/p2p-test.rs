use libp2p::identity;
use libp2p::noise;
use libp2p::request_response;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::NetworkBehaviour;
use libp2p::yamux;
use libp2p::StreamProtocol;
use libp2p::Swarm;
use std::error::Error;
use std::time::Duration;
use wormhole::network::codec::BincodeCodec;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GreetRequest {
    name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GreetResponse {
    message: String,
}

#[derive(NetworkBehaviour)]
struct Network {
    request_response: request_response::Behaviour<BincodeCodec>,
}

async fn create_swarm() -> Result<Swarm<Network>, Box<dyn Error>> {
    let pod_name = String::from("virtual");
    let bytes: Vec<u8> = pod_name.bytes().collect();

    let id_keys = identity::Keypair::ed25519_from_bytes(bytes).unwrap();

    let swarm = libp2p::SwarmBuilder::with_existing_identity(id_keys)
        .with_tokio()
        .with_websocket(noise::Config::new, yamux::Config::default)
        .await?
        .with_behaviour(|_| Network {
            request_response: request_response::Behaviour::new(
                [(StreamProtocol::new("/wormhole/1"), ProtocolSupport::Full)],
                request_response::Config::default(), // Possible configuration file later
            ),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    Ok(swarm)
}

#[tokio::main]
async fn main() {
    let swarm = create_swarm().await.unwrap();
}
