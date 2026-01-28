use crate::pods::network::behaviour::Behaviour;
use libp2p::{
    identify, noise,
    request_response::{self, ProtocolSupport},
    yamux, StreamProtocol, Swarm,
};
use std::{error::Error, time::Duration};

const PROTOCOL_VERSION: &str = "/wormhole/1.0.0";

pub const MAX_CONCURRENT_STREAMS: usize = 128;

pub async fn create_swarm(nickname: String) -> Result<Swarm<Behaviour>, Box<dyn Error>> {
    let swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_websocket(noise::Config::new, yamux::Config::default)
        .await?
        .with_behaviour(|key| {
            let cfg = identify::Config::new(PROTOCOL_VERSION.to_string(), key.public())
                .with_agent_version(nickname);

            Behaviour {
                request_response: request_response::Behaviour::new(
                    [(StreamProtocol::new(PROTOCOL_VERSION), ProtocolSupport::Full)],
                    request_response::Config::default()
                        .with_max_concurrent_streams(MAX_CONCURRENT_STREAMS),
                ),
                identify: identify::Behaviour::new(cfg),
            }
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
        .build();

    Ok(swarm)
}
