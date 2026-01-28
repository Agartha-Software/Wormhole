use libp2p::{identify, request_response, swarm};

use crate::pods::network::codec::BincodeCodec;

#[derive(swarm::NetworkBehaviour)]
pub struct Behaviour {
    pub request_response: request_response::Behaviour<BincodeCodec>,
    pub identify: identify::Behaviour,
}
