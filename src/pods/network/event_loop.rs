use std::{collections::HashMap, io, sync::Arc};

use futures::StreamExt;
use libp2p::{
    identify,
    request_response::{self, OutboundRequestId, ResponseChannel},
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use tokio::sync::{mpsc::UnboundedReceiver, oneshot};

use crate::{
    ipc::answers::PeerInfo,
    network::message::{Request, Response, ToNetworkMessage},
    pods::{
        filesystem::fs_interface::FsInterface,
        itree::creation::initiate_itree,
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
    answers: HashMap<OutboundRequestId, oneshot::Sender<Option<Response>>>,
    closing: bool,
    need_initialisation: Option<Option<OutboundRequestId>>,
}

impl EventLoop {
    pub fn new(
        swarm: Swarm<Behaviour>,
        fs_interface: Arc<FsInterface>,
        to_network: UnboundedReceiver<ToNetworkMessage>,
        need_initialisation: bool,
    ) -> Self {
        EventLoop {
            swarm,
            to_network,
            fs_interface,
            answers: HashMap::new(),
            closing: false,
            need_initialisation: if need_initialisation {
                Some(None)
            } else {
                None
            },
        }
    }

    fn close(mut self) {
        self.closing = true;
        for peer in self
            .swarm
            .connected_peers()
            .cloned()
            .collect::<Vec<PeerId>>()
        {
            self.swarm
                .behaviour_mut()
                .request_response
                .send_request(&peer, Request::Disconnect);
            let _ = self.swarm.disconnect_peer_id(peer);
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => if self.handle_event(event).await {
                    return
                },
                to_network = self.to_network.recv() => match to_network {
                    Some(ToNetworkMessage::AnswerMessage(message, status, peer)) => self.send_with_answer(message, status, peer),
                    Some(ToNetworkMessage::SpecificMessage(message, to)) => self.send_to_multiple(message, to),
                    Some(ToNetworkMessage::BroadcastMessage(message)) => self.send_to_multiple(message, self.swarm.connected_peers().cloned().collect()),
                    Some(ToNetworkMessage::CloseNetwork) => {
                        self.close();
                        return;
                    }
                    None => {
                        self.close();
                        return;
                    },
                }
            }
        }
    }

    fn send_with_answer(
        &mut self,
        message: Request,
        status: oneshot::Sender<Option<Response>>,
        peer: PeerId,
    ) {
        let answer = self
            .swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer, message);
        self.answers.insert(answer, status);
    }

    fn send_to_multiple(&mut self, message: Request, mut to: Vec<PeerId>) {
        if let Some(last) = to.pop() {
            // Just to don't clone the message on first peer, lot's of message have only one peer and messages can be very heavy quickly
            for peer in to {
                self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, message.clone());
            }

            self.swarm
                .behaviour_mut()
                .request_response
                .send_request(&last, message);
        }
    }

    fn retry_fs_request(&mut self, failing_host: PeerId) {
        let retry_peer = self
            .swarm
            .connected_peers()
            .find(|peer| **peer != failing_host)
            .unwrap_or(&failing_host)
            .clone();

        let request_id = self
            .swarm
            .behaviour_mut()
            .request_response
            .send_request(&retry_peer, Request::RequestFs);

        self.need_initialisation = Some(Some(request_id));
    }

    async fn handle_response_message(&mut self, response: Response, peer: PeerId) {
        log::trace!("Network Request: {:?}", response);

        let result = match response {
            Response::DeltaRequest(ino, sig) => self
                .fs_interface
                .respond_delta(ino, sig, peer)
                .map_err(into_boxed_io),
            Response::FsAnswer(tree, peers, global_config) => {
                self.need_initialisation = None;

                for peer in peers {
                    log::trace!("Trying to connect to the other peer: {peer}");
                    let _ = self.swarm.dial(peer).inspect_err(|e| {
                        log::error!("Could'nt connect to {peer} on the network: {e}")
                    });
                }

                let mut current = self.fs_interface.network_interface.itree.write();
                // Overwrite local tree
                *current = tree;

                if let Err(err) =
                    initiate_itree(&current, &global_config, self.fs_interface.disk.as_ref())
                {
                    log::error!("New itree failed: {err}, asking for an other");
                    drop(current);
                    self.retry_fs_request(peer);
                }
                Ok(())
            }
            _ => Ok(()),
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
        log::trace!("Network Request: {:?}", request);
        let result = match request {
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
            Request::RequestFile(inode) => {
                self.fs_interface.send_file(inode).map_err(into_boxed_io)
            }
            Request::RequestFs => self
                .fs_interface
                .network_interface
                .send_filesystem(peer)
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
                .accept_delta(ino, meta, sig, delta)
                .map_err(into_boxed_io),
            Request::FileChanged(ino, meta) => self
                .fs_interface
                .accept_file_changed(ino, meta)
                .map_err(into_boxed_io),
        };

        match result {
            Ok(response) => {
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, response);
            }
            Err(err) => {
                let _ = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, Response::Failed);
                log::trace!("Request Message Failed: {err}")
            }
        };
    }

    async fn handle_rr_event(&mut self, event: request_response::Event<Request, Response>) {
        match event {
            request_response::Event::Message { peer, message, .. } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => self.handle_request_message(request, channel, peer).await,
                request_response::Message::Response {
                    response,
                    request_id,
                } => {
                    if let Some(answer) = self.answers.remove(&request_id) {
                        let _ = answer.send(Some(response.clone()));
                    };
                    self.handle_response_message(response, peer).await;
                }
            },
            request_response::Event::OutboundFailure {
                peer, request_id, ..
            } => {
                if let Some(Some(id)) = self.need_initialisation {
                    if id == request_id {
                        self.retry_fs_request(peer);
                        return;
                    }
                }
                if let Some(answer) = self.answers.remove(&request_id) {
                    let _ = answer.send(None);
                }
            }
            e => log::trace!("rr: {e:?}"),
        }
    }

    fn handle_identify_event(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received {
                connection_id,
                peer_id,
                info,
            } => {
                log::trace!("id received!: {} {} {:?}", connection_id, peer_id, info);
                if let Some(None) = self.need_initialisation {
                    let request_id = self
                        .swarm
                        .behaviour_mut()
                        .request_response
                        .send_request(&peer_id, Request::RequestFs);
                    self.need_initialisation = Some(Some(request_id));
                };
                self.fs_interface
                    .network_interface
                    .peers_info
                    .write()
                    .insert(
                        peer_id,
                        PeerInfo {
                            name: info.agent_version,
                            listen_addrs: info.listen_addrs,
                        },
                    );
            }
            e => log::trace!("identify: {e:?}"),
        }
    }

    pub async fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent>) -> bool {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(event)) => {
                self.handle_rr_event(event).await;
                ()
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(event)) => {
                self.handle_identify_event(event);
                ()
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                self.fs_interface
                    .network_interface
                    .listen_addrs
                    .write()
                    .insert(address.clone());
                log::trace!("new listen address: {address:?}")
            }
            SwarmEvent::ExpiredListenAddr { address, .. } => {
                self.fs_interface
                    .network_interface
                    .listen_addrs
                    .write()
                    .remove(&address);
                log::trace!("expired listen address: {address:?}")
            }
            SwarmEvent::ConnectionEstablished { .. } => {
                // Peer interaction start at identify
            }
            SwarmEvent::IncomingConnection { connection_id, .. } => {
                if self.closing {
                    self.swarm.close_connection(connection_id);
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                if self.closing && self.swarm.connected_peers().count() == 0 {
                    return true;
                }
                self.fs_interface
                    .network_interface
                    .peers_info
                    .write()
                    .remove(&peer_id);
            }
            e => log::trace!("event: {e:?}"),
        };
        return false;
    }
}
