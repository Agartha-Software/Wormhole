use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    task::Poll,
};

use libp2p::{
    core::{transport, Endpoint},
    request_response::{self, Codec, InboundRequestId, OutboundRequestId},
    swarm::{self, NetworkBehaviour},
    Multiaddr, PeerId,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub struct Behaviour<TCodec: Codec + Clone + Send + 'static> {
    request_response: request_response::Behaviour<TCodec>,
    permits: HashMap<OutboundRequestId, OwnedSemaphorePermit>,
    semaphore: Arc<Semaphore>,
    inbound: HashSet<InboundRequestId>,
}

impl<TCodec: Codec + Clone + Send + 'static> NetworkBehaviour for Behaviour<TCodec> {
    type ConnectionHandler =
        <request_response::Behaviour<TCodec> as NetworkBehaviour>::ConnectionHandler;

    type ToSwarm = <request_response::Behaviour<TCodec> as NetworkBehaviour>::ToSwarm;

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: swarm::ConnectionId,
        peer: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<swarm::THandler<Self>, swarm::ConnectionDenied> {
        self.request_response.handle_established_inbound_connection(
            connection_id,
            peer,
            local_addr,
            remote_addr,
        )
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: swarm::ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: Endpoint,
        port_use: transport::PortUse,
    ) -> Result<swarm::THandler<Self>, swarm::ConnectionDenied> {
        self.request_response
            .handle_established_outbound_connection(
                connection_id,
                peer,
                addr,
                role_override,
                port_use,
            )
    }

    fn on_swarm_event(&mut self, event: swarm::FromSwarm) {
        self.request_response.on_swarm_event(event);
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: swarm::ConnectionId,
        event: swarm::THandlerOutEvent<Self>,
    ) {
        self.request_response
            .on_connection_handler_event(peer_id, connection_id, event);
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<swarm::ToSwarm<Self::ToSwarm, swarm::THandlerInEvent<Self>>> {
        let poll = self.request_response.poll(cx);
        if let Poll::Ready(to_swarm) = &poll {
            match to_swarm {
                swarm::ToSwarm::GenerateEvent(event) => match event {
                    request_response::Event::Message { message, .. } => match message {
                        request_response::Message::Request { request_id, .. } => {
                            self.inbound.insert(*request_id);
                        }
                        request_response::Message::Response { request_id, .. } => {
                            self.permits.remove(request_id);
                        }
                    },
                    request_response::Event::OutboundFailure { request_id, .. } => {
                        self.permits.remove(request_id);
                    }
                    request_response::Event::InboundFailure { request_id, .. } => {
                        self.inbound.remove(request_id);
                    }
                    request_response::Event::ResponseSent { request_id, .. } => {
                        self.inbound.remove(request_id);
                    }
                },
                _ => {}
            }
        }
        poll
    }
}

use libp2p::request_response::{Config, ProtocolSupport, ResponseChannel};

use crate::pods::network::swarm::{MAX_CONCURRENT_CONNECTIONS, MAX_CONCURRENT_STREAMS_OVERHEAD};

impl<TCodec> Behaviour<TCodec>
where
    TCodec: Codec + Default + Clone + Send + 'static,
{
    pub fn new<I>(protocols: I, cfg: Config) -> Self
    where
        I: IntoIterator<Item = (TCodec::Protocol, ProtocolSupport)>,
    {
        Self::with_codec(TCodec::default(), protocols, cfg)
    }
}

impl<TCodec> Behaviour<TCodec>
where
    TCodec: Codec + Clone + Send + 'static,
{
    pub fn with_codec<I>(codec: TCodec, protocols: I, cfg: Config) -> Self
    where
        I: IntoIterator<Item = (TCodec::Protocol, ProtocolSupport)>,
    {
        Self {
            request_response: request_response::Behaviour::with_codec(
                codec,
                protocols,
                cfg.with_max_concurrent_streams(
                    MAX_CONCURRENT_CONNECTIONS + MAX_CONCURRENT_STREAMS_OVERHEAD,
                ),
            ),
            permits: Default::default(),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS)),
            inbound: Default::default(),
        }
    }
}

/// Implementations of semaphore limiting.
impl<TCodec> Behaviour<TCodec>
where
    TCodec: Codec + Clone + Send + 'static,
{
    pub fn send_request(
        &mut self,
        permit: OwnedSemaphorePermit,
        peer: &PeerId,
        request: TCodec::Request,
    ) -> OutboundRequestId {
        self.send_request_with_addresses(permit, peer, request, Vec::new())
    }

    pub fn send_request_with_addresses(
        &mut self,
        permit: OwnedSemaphorePermit,
        peer: &PeerId,
        request: TCodec::Request,
        addresses: Vec<Multiaddr>,
    ) -> OutboundRequestId {
        let request_id = self
            .request_response
            .send_request_with_addresses(peer, request, addresses);
        self.permits.insert(request_id, permit);
        request_id
    }

    /// cheat and generate a 0-sized permit
    /// may only be used for special cases that can't be arbitrarily multiplied
    pub fn nopermit(&mut self) -> OwnedSemaphorePermit {
        Semaphore::try_acquire_many_owned(self.semaphore.clone(), 0)
            .expect("aquiring 0 permits should never fail because the semaphore never closes")
    }

    pub fn permit(&mut self) -> Option<OwnedSemaphorePermit> {
        Semaphore::try_acquire_owned(self.semaphore.clone()).ok()
    }

    pub fn permits(&mut self, n: u32) -> Option<Vec<OwnedSemaphorePermit>> {
        let Some(mut p) = Semaphore::try_acquire_many_owned(self.semaphore.clone(), n).ok() else {
            return None;
        };
        Some(
            (0..n)
                .into_iter()
                .map(|_| p.split(1).expect("known to be enough"))
                .collect(),
        )
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }
}

impl<TCodec> Behaviour<TCodec>
where
    TCodec: Codec + Clone + Send + 'static,
{
    pub fn send_response(
        &mut self,
        ch: ResponseChannel<TCodec::Response>,
        rs: TCodec::Response,
    ) -> Result<(), TCodec::Response> {
        self.request_response.send_response(ch, rs)
    }

    pub fn is_connected(&self, peer: &PeerId) -> bool {
        self.request_response.is_connected(peer)
    }

    pub fn is_pending_outbound(&self, peer: &PeerId, request_id: &OutboundRequestId) -> bool {
        self.request_response.is_pending_outbound(peer, request_id)
    }

    pub fn is_pending_inbound(&self, peer: &PeerId, request_id: &InboundRequestId) -> bool {
        self.request_response.is_pending_inbound(peer, request_id)
    }
}
