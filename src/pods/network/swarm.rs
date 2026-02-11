use crate::{network::quota_req_res, pods::network::behaviour::Behaviour};
use libp2p::{
    identify, noise,
    request_response::{self, ProtocolSupport},
    yamux, StreamProtocol, Swarm,
};
use std::{error::Error, time::Duration};

const PROTOCOL_VERSION: &str = "/wormhole/1.0.0";

/// Maximum number of connections the swarm voluntarily creates
pub const MAX_CONCURRENT_CONNECTIONS: usize = 128;

/// Overhead of streams to ensure that the hard limit is never reached, even if connections created by peers
pub const MAX_CONCURRENT_STREAMS_OVERHEAD: usize = 128;

pub async fn create_swarm(nickname: String) -> Result<Swarm<Behaviour>, Box<dyn Error>> {
    let swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_websocket(noise::Config::new, yamux::Config::default)
        .await?
        .with_behaviour(|key| {
            let cfg = identify::Config::new(PROTOCOL_VERSION.to_string(), key.public())
                .with_agent_version(nickname);

            Behaviour {
                request_response: quota_req_res::Behaviour::new(
                    [(StreamProtocol::new(PROTOCOL_VERSION), ProtocolSupport::Full)],
                    request_response::Config::default(),
                ),
                identify: identify::Behaviour::new(cfg),
            }
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();

    Ok(swarm)
}
