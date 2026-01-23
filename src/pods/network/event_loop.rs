use std::{io, sync::Arc};

use futures::StreamExt;
use libp2p::{
    identify,
    request_response::{self, ResponseChannel},
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    network::message::{Request, Response, ToNetworkMessage},
    pods::{
        filesystem::fs_interface::FsInterface,
        network::behaviour::{Behaviour, BehaviourEvent},
    },
};

// We use a function here because we need templates, but we don't want to leak this kind of weird function to anywhere else
fn into_boxed_io<T: std::error::Error>(err: T) -> io::Error {
    std::io::Error::other(format!("{}: {err}", std::any::type_name::<T>()))
}

pub struct EventLoop {
    swarm: Swarm<Behaviour>,
    fs_interface: Arc<FsInterface>,
    to_network: UnboundedReceiver<ToNetworkMessage>,
}

pub struct ResponseSender<'a> {
    behavior: &'a mut Behaviour,
    channel: ResponseChannel<Response>,
}

impl<'a> ResponseSender<'a> {
    pub fn send(self, response: Response) {
        self.behavior
            .request_response
            .send_response(self.channel, response);
    }

    pub fn new(behavior: &'a mut Behaviour, channel: ResponseChannel<Response>) -> Self {
        Self { behavior, channel }
    }
}

impl EventLoop {
    pub fn new(
        swarm: Swarm<Behaviour>,
        fs_interface: Arc<FsInterface>,
        to_network: UnboundedReceiver<ToNetworkMessage>,
    ) -> Self {
        EventLoop {
            swarm,
            to_network,
            fs_interface,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                to_network = self.to_network.recv() => match to_network {
                    Some(ToNetworkMessage::SpecificMessage((message, _), _)) => self.send_request(message),
                    Some(ToNetworkMessage::BroadcastMessage(message)) => self.send_request(message),
                    None => return,
                }
            }
        }
    }

    fn send_request(&mut self, message: Request) {
        log::error!("CANT SEND MESSAGE YET: {}", message);
    }

    async fn handle_response_message(&mut self, response: Response, peer: PeerId) {
        let result = match response {
            Response::DeltaRequest(ino, sig) => self
                .fs_interface
                .respond_delta(ino, sig, peer)
                .map_err(into_boxed_io),
            e => {
                log::trace!("Recieved fsAnswer {}", e);
                Ok(())
            }
        };
        if let Err(err) = result {
            log::trace!("Response Message Failed: {err}");
        }
    }

    async fn handle_request_message(
        &mut self,
        request: Request,
        channel: ResponseChannel<Response>,
        peer: PeerId,
    ) {
        let sender = ResponseSender::new(self.swarm.behaviour_mut(), channel);

        let result = match request {
            Request::PullAnswer(id, binary) => self
                .fs_interface
                .recept_binary(id, binary)
                .map_err(into_boxed_io),
            Request::RedundancyFile(id, binary) => self
                .fs_interface
                .recept_redundancy(id, binary)
                .map_err(into_boxed_io),
            Request::Inode(inode) => self.fs_interface.recept_inode(inode).map_err(into_boxed_io),
            Request::EditHosts(id, hosts) => self
                .fs_interface
                .recept_edit_hosts(id, hosts)
                .map_err(into_boxed_io),
            Request::RevokeFile(id, host, meta) => self
                .fs_interface
                .recept_revoke_hosts(id, host, meta)
                .map_err(into_boxed_io),
            Request::AddHosts(id, hosts) => self
                .fs_interface
                .recept_add_hosts(id, hosts)
                .map_err(into_boxed_io),
            Request::RemoveHosts(id, hosts) => self
                .fs_interface
                .recept_remove_hosts(id, hosts)
                .map_err(into_boxed_io),
            Request::EditMetadata(id, meta) => self
                .fs_interface
                .acknowledge_metadata(id, meta)
                .map_err(into_boxed_io),
            Request::Remove(id) => self
                .fs_interface
                .recept_remove_inode(id)
                .map_err(into_boxed_io),
            Request::RequestFile(inode) => self
                .fs_interface
                .send_file(inode, peer)
                .map_err(into_boxed_io),
            Request::RequestFs => self
                .fs_interface
                .send_filesystem(peer, sender)
                .map_err(into_boxed_io),
            Request::Rename(parent, new_parent, name, new_name, overwrite) => self
                .fs_interface
                .recept_rename(parent, new_parent, name, new_name, overwrite)
                .map_err(into_boxed_io),
            Request::SetXAttr(ino, key, data) => self
                .fs_interface
                .network_interface
                .recept_inode_xattr(ino, &key, data)
                .map_err(into_boxed_io),

            Request::RemoveXAttr(ino, key) => self
                .fs_interface
                .network_interface
                .recept_remove_inode_xattr(ino, &key)
                .map_err(into_boxed_io),
            Request::Disconnect => self
                .fs_interface
                .network_interface
                .disconnect_peer(peer)
                .map_err(into_boxed_io),
            Request::FileDelta(ino, meta, sig, delta) => self
                .fs_interface
                .accept_delta(ino, meta, sig, delta, sender)
                .map_err(into_boxed_io),
            Request::FileChanged(ino, meta) => self
                .fs_interface
                .accept_file_changed(ino, meta, sender)
                .map_err(into_boxed_io),
        };

        if let Err(err) = result {
            log::trace!("Request Message Failed: {err}");
        }
    }

    async fn handle_rr_event(&mut self, event: request_response::Event<Request, Response>) {
        match event {
            request_response::Event::Message {
                peer,
                connection_id: _,
                message,
            } => match message {
                request_response::Message::Request {
                    request_id: _,
                    request,
                    channel,
                } => self.handle_request_message(request, channel, peer).await,
                request_response::Message::Response {
                    request_id: _,
                    response,
                } => self.handle_response_message(response, peer).await,
            },
            e => log::trace!("rr: {e:?}"),
        }
    }

    async fn handle_identify_event(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received {
                connection_id,
                peer_id,
                info,
            } => log::trace!("id received!: {} {} {:?}", connection_id, peer_id, info),
            e => log::trace!("identify: {e:?}"),
        }
    }

    pub async fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(event)) => {
                self.handle_rr_event(event).await
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(event)) => {
                self.handle_identify_event(event).await
            }
            e => log::trace!("event: {e:?}"),
        }
    }
}
