use libp2p::{identify, swarm};

use crate::network::quota_req_res;
use crate::pods::network::codec::BincodeCodec;

#[derive(swarm::NetworkBehaviour)]
pub struct Behaviour {
    pub request_response: quota_req_res::Behaviour<BincodeCodec>,
    // pub request_response: request_response::Behaviour<BincodeCodec>,
    pub identify: identify::Behaviour,
}
