use libp2p::{identify, swarm};

use crate::pods::network::codec::BincodeCodec;
use crate::network::quota_req_res;

#[derive(swarm::NetworkBehaviour)]
pub struct Behaviour {
    pub request_response: quota_req_res::Behaviour<BincodeCodec>,
    // pub request_response: request_response::Behaviour<BincodeCodec>,
    pub identify: identify::Behaviour,
}
