use crate::pods::network::behaviour::Behaviour;
use libp2p::{
    identity, noise,
    request_response::{self, ProtocolSupport},
    yamux, StreamProtocol, Swarm,
};
use std::{error::Error, time::Duration};

pub async fn create_swarm(key: String) -> Result<Swarm<Behaviour>, Box<dyn Error>> {
    let bytes: Vec<u8> = key.bytes().collect();

    let id_keys = identity::Keypair::ed25519_from_bytes(bytes).unwrap();

    let swarm = libp2p::SwarmBuilder::with_existing_identity(id_keys)
        .with_tokio()
        .with_websocket(noise::Config::new, yamux::Config::default)
        .await?
        .with_behaviour(|_| Behaviour {
            request_response: request_response::Behaviour::new(
                [(StreamProtocol::new("/wormhole/1"), ProtocolSupport::Full)],
                request_response::Config::default(),
            ),
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    Ok(swarm)
}
