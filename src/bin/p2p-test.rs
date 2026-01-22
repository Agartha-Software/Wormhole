use async_trait::async_trait;
use futures::prelude::*;
use libp2p::identity;
use libp2p::noise;
use libp2p::request_response;
use libp2p::request_response::Codec;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::NetworkBehaviour;
use libp2p::yamux;
use libp2p::StreamProtocol;
use libp2p::Swarm;
use std::error::Error;
use std::io;
use std::time::Duration;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GreetRequest {
    name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GreetResponse {
    message: String,
}

#[derive(Clone, Default)]
pub struct BincodeCodec {}

#[async_trait]
impl Codec for BincodeCodec {
    type Protocol = StreamProtocol;
    type Request = GreetRequest;
    type Response = GreetResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        todo!();
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        todo!();
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        todo!();
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        todo!();
    }
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
        .with_behaviour(|key| Network {
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
