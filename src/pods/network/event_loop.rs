use std::{
    collections::HashMap,
    io,
    ops::Deref,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::Arc,
    thread::panicking,
    time::Duration,
};

use futures::{stream::Peekable, StreamExt as _};
use libp2p::{
    identify,
    request_response::{self, OutboundRequestId, ResponseChannel},
    swarm::{dial_opts::DialOpts, ConnectionError, SwarmEvent},
    PeerId, Swarm,
};
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedReceiver, oneshot, OwnedSemaphorePermit},
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
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
    to_network: Peekable<UnboundedReceiverStream<ToNetworkMessage>>,
    answers: HashMap<OutboundRequestId, oneshot::Sender<Option<Response>>>,
    closing: bool,
    need_initialisation: Option<Option<OutboundRequestId>>,
}

/// Ensure that when panicking, this pod tells other peers that it's leaving the network
///
/// REVIEW: Should we set a specif Request::Crashing message ? I don't see what peers could do about it,
///  but it feels underhanded to say simply 'Leaving' when in fact, it's not fine
///
/// This is a bit of a risky operation because we don't have access to our async context,
/// panicking again will break the natural unwind process,
/// and we don't want to hang this thread.
/// We've wrapped the sensitive part in a catch_unwind to protect against double panicking
/// We're blocking for at most 5 seconds to ensure the messages are sent.
impl Drop for EventLoop {
    fn drop(&mut self) {
        const DROP_LEAVE_TIMEOUT: u64 = 5;
        if !panicking() {
            return;
        }
        if !self.closing {
            self.leave();
        }

        // Safety: We don't observe any invariants on self after the caught panic
        // All we're doing afterwards is dropping Self and its components,
        // which is a natural thing to do and already guaranteed to be safe
        let mut safe = AssertUnwindSafe(self);

        let maybe_unwinding = move || {
            tokio::task::block_in_place(move || {
                log::trace!(
                    "Drop: Sending Leave to all peers; waiting {DROP_LEAVE_TIMEOUT} seconds..."
                );
                let res = Handle::current().block_on(tokio::time::timeout(
                    Duration::from_secs(DROP_LEAVE_TIMEOUT),
                    safe.run(),
                ));
                match res {
                    Ok(()) => log::trace!("Drop: All peers acknowledged"),
                    Err(_) => log::error!("Drop: Not peers acknowledged"),
                }
            });
        };

        let res = catch_unwind(maybe_unwinding);
        if let Err(e) = res {
            log::error!("Drop: Double panic while dropping: {e:#?}");
        }
    }
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
            to_network: futures::StreamExt::peekable(UnboundedReceiverStream::new(to_network)),
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

    /// Tells all connected peers that we are leaving the network
    /// This creates an answer for each peer, ensuring that we send the event to
    /// all known peers before shutting down
    ///
    /// It is considered invalid to process any Behavior after the Leave event has been sent,
    /// that is because locally and on remotes, the Leave is performed immediately and peers are removed from the list
    /// this pod will not recognize any peers,
    /// remotes will not recognize this peer after it has left,
    fn leave(&mut self) {
        log::debug!("Leave: ejecting all peers");
        log::trace!(
            "Leave: {} requests pending at time of closing",
            self.answers.len()
        );
        self.closing = true;

        let drain = self
            .fs_interface
            .network_interface
            .peers_info
            .write()
            .drain()
            .collect::<Vec<_>>();

        for (peer, _) in drain {
            let (status, recv) = oneshot::channel();
            let permit = self.swarm.behaviour_mut().request_response.nopermit();
            self.send_with_answer(permit, Request::Leave, status, peer);
            drop(recv); // we don't care about the answer, we just want it to be created;
        }
    }

    pub async fn run(&mut self) {
        while !self.closing || !self.answers.is_empty() {
            if self.closing {
                log::trace!(
                    "Run: Closing but answers remain: {:?}",
                    &self
                        .answers
                        .keys()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                );
            }
            tokio::select! {
                biased; // This forces tokio to respect the order specified: we want to close only if no network event are pending
                event = self.swarm.select_next_some() => if self.handle_event(event) {
                    return
                },
                to_network = Peekable::<UnboundedReceiverStream<ToNetworkMessage>>::peek(Pin::new(&mut self.to_network)), if !self.closing => {
                    let peers = self
                        .fs_interface
                        .network_interface
                        .peers_info
                        .read()
                        .keys()
                        .copied()
                        .collect::<Vec<_>>();
                    let permits = match to_network {
                        Some(ToNetworkMessage::AnswerMessage(..)) => 0,
                        Some(ToNetworkMessage::SpecificMessage(_, to)) => to.len() as u32,
                        Some(ToNetworkMessage::BroadcastMessage(_)) => peers.len() as u32,
                        _ => 0,
                    };

                    if let Some(mut permits) = self.swarm.behaviour_mut().request_response.permits(permits) {
                        let next = self.to_network.next().await;

                        match next {
                            Some(ToNetworkMessage::AnswerMessage(message, status, peer)) => self
                                .send_with_answer(
                                    permits.pop().expect("permits is non-empty"),
                                    message,
                                    status,
                                    peer,
                                ),
                            Some(ToNetworkMessage::SpecificMessage(message, to)) => {
                                self.send_to_multiple(permits, message, &to)
                            }
                            Some(ToNetworkMessage::BroadcastMessage(message)) => {
                                self.send_to_multiple(permits, message, &peers)
                            }
                            Some(ToNetworkMessage::LeaveNetwork) => {
                                self.leave();
                            }
                            None => {
                                self.leave();
                            }
                        }
                    }
                }
            }
        }
    }

    fn send_with_answer(
        &mut self,
        permit: OwnedSemaphorePermit,
        message: Request,
        status: oneshot::Sender<Option<Response>>,
        peer: PeerId,
    ) {
        let log_msg = log::log_enabled!(log::Level::Trace).then_some(format!("{message}"));
        let request_id = self
            .swarm
            .behaviour_mut()
            .request_response
            .send_request(permit, &peer, message);
        if let Some(log_msg) = log_msg {
            log::trace!("Requesting {log_msg} to {peer}: #{request_id}");
        }
        self.answers.insert(request_id, status);
    }

    fn send_to_multiple<I: IntoIterator<Item = impl Deref<Target = PeerId>> + std::fmt::Debug>(
        &mut self,
        permits: impl IntoIterator<Item = OwnedSemaphorePermit>,
        message: Request,
        to: I,
    ) {
        let mut log_to = vec![];
        let to = to.into_iter();
        let permits = permits.into_iter();

        let mut zip = to.zip(permits);
        // avoid cloning the message an extra time. put aside the first send
        if let Some((first, first_permit)) = zip.next() {
            if log::log_enabled!(log::Level::Debug) {
                log_to.push(first.to_base58());
            }
            for (peer, permit) in zip {
                self.swarm.behaviour_mut().request_response.send_request(
                    permit,
                    peer.deref(),
                    message.clone(),
                );
                if log::log_enabled!(log::Level::Debug) {
                    log_to.push(peer.to_base58());
                }
            }

            if log_to.len() > 1 {
                log::debug!("Broadcasting {message} to {:?}", &log_to[..]);
            } else {
                log::debug!("Sending {message} to {:?}", first.to_base58());
            }

            // let it be moved in here
            self.swarm.behaviour_mut().request_response.send_request(
                first_permit,
                first.deref(),
                message,
            );
        }
    }

    fn retry_fs_request(&mut self, failing_host: PeerId) {
        let retry_peer = *self
            .swarm
            .connected_peers()
            .find(|peer| **peer != failing_host)
            .unwrap_or(&failing_host);
        let permit = self.swarm.behaviour_mut().request_response.nopermit();
        let request_id = self.swarm.behaviour_mut().request_response.send_request(
            permit,
            &retry_peer,
            Request::RequestFs,
        );

        self.need_initialisation = Some(Some(request_id));
    }

    fn handle_response_message(&mut self, response: Response, peer: PeerId) {
        let result = match response {
            Response::DeltaRequest(ino, sig) => self
                .fs_interface
                .respond_delta(ino, sig, peer)
                .map_err(into_boxed_io),
            Response::FsAnswer(tree, peers, global_config) => {
                self.need_initialisation = None;

                {
                    let mut peers_info = self.fs_interface.network_interface.peers_info.write();
                    for (peer, info) in peers {
                        peers_info.insert(peer.clone(), info.clone());
                        log::trace!(
                            "Join: Registering address to the peer: {peer}: {:?}",
                            info.listen_addrs
                        );
                        for addr in info.listen_addrs {
                            self.swarm.add_peer_address(peer, addr);
                        }
                        let dial = self.swarm.dial(DialOpts::peer_id(peer).build());
                        if let Err(error) = dial {
                            log::error!("Join: Failed to dial {peer}: {error:?}");
                        }
                    }
                }

                let mut current = self.fs_interface.network_interface.itree.write();
                // Overwrite local tree
                *current = tree;

                if let Err(err) =
                    initiate_itree(&current, &global_config, self.fs_interface.disk.as_ref())
                {
                    log::error!("Join: New itree failed: {err}, asking for an other");
                    drop(current);
                    self.retry_fs_request(peer);
                }
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(err) = result {
            log::trace!("Response: Processing Failed: {err}");
        }
    }

    fn handle_request_message(
        &mut self,
        request: Request,
        channel: ResponseChannel<Response>,
        peer: PeerId,
    ) {
        let result = match request {
            Request::RedundancyFile(id, binary) => self
                .fs_interface
                .recept_redundancy(id, binary)
                .map_err(into_boxed_io),
            Request::Inode(inode) => self.fs_interface.recept_inode(inode).map_err(into_boxed_io),
            Request::AddHosts(id, hosts) => self
                .fs_interface
                .recept_add_hosts(id, &hosts)
                .map_err(into_boxed_io),
            Request::RemoveHosts(id, hosts) => self
                .fs_interface
                .recept_remove_hosts(id, &hosts)
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
            Request::FileDelta(ino, meta, sig, delta) => self
                .fs_interface
                .accept_delta(ino, meta, sig, delta)
                .map_err(into_boxed_io),
            Request::FileChanged(ino, meta) => self
                .fs_interface
                .accept_file_changed(ino, meta)
                .map_err(into_boxed_io),
            Request::Leave => self
                .fs_interface
                .network_interface
                .disconnect_peer(peer)
                .map_err(into_boxed_io),
            Request::Bannish(peer_id) => {
                let _ = self.swarm.disconnect_peer_id(peer_id);
                self.fs_interface
                    .network_interface
                    .disconnect_peer(peer_id)
                    .map_err(into_boxed_io)
            }
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
                log::trace!("Request: Processsing failed: {err}")
            }
        };
    }

    fn handle_rr_event(&mut self, event: request_response::Event<Request, Response>) {
        match event {
            request_response::Event::Message { peer, message, .. } => match message {
                request_response::Message::Request {
                    request,
                    channel,
                    request_id,
                } => {
                    if !self.closing {
                        log::trace!("Got Request: {:?} : #{} ", request, request_id);
                        self.handle_request_message(request, channel, peer)
                    } else {
                        log::trace!(
                            "Got Request: {:?} : #{} (ignored while closing)",
                            request,
                            request_id
                        );
                    }
                }
                request_response::Message::Response {
                    response,
                    request_id,
                } => {
                    if let Some(answer) = self.answers.remove(&request_id) {
                        let _ = answer.send(Some(response.clone()));
                    };
                    if !self.closing {
                        log::trace!("Got Response: {:?} : #{}", response, request_id);
                        self.handle_response_message(response, peer);
                    } else {
                        log::trace!(
                            "Got Response: {:?} : #{} (ignored while closing)",
                            response,
                            request_id
                        );
                    }
                }
            },
            request_response::Event::ResponseSent { request_id, .. } => {
                log::trace!("Event: Response sent: #{request_id}")
            }
            request_response::Event::OutboundFailure {
                peer,
                request_id,
                error,
                ..
            } => {
                log::error!("Event: Outbound Failure: #{request_id} : {error}");
                if let Some(answer) = self.answers.remove(&request_id) {
                    let _ = answer.send(None);
                }
                if let Some(Some(id)) = self.need_initialisation {
                    if id == request_id && !self.closing {
                        self.retry_fs_request(peer);
                        return;
                    }
                }
            }
            request_response::Event::InboundFailure {
                request_id, error, ..
            } => {
                log::error!("Event: Inbound Failure:  #{request_id} : {error}");
            }
        }
    }

    fn handle_identify_event(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received { peer_id, info, .. } => {
                log::trace!("Identify: {}: {:?}", peer_id, info);
                if let Some(None) = self.need_initialisation {
                    let permit = self.swarm.behaviour_mut().request_response.nopermit();
                    let request_id = self.swarm.behaviour_mut().request_response.send_request(
                        permit,
                        &peer_id,
                        Request::RequestFs,
                    );
                    self.need_initialisation = Some(Some(request_id));
                };
                self.fs_interface
                    .network_interface
                    .connect_peer(peer_id, info);
            }
            identify::Event::Sent { .. } => {}
            e => log::trace!("Identify: {e:?}"),
        }
    }

    pub fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent>) -> bool {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(event)) => {
                self.handle_rr_event(event)
            }
            SwarmEvent::Behaviour(BehaviourEvent::Identify(event)) => {
                self.handle_identify_event(event)
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                self.fs_interface
                    .network_interface
                    .listen_addrs
                    .write()
                    .insert(address.clone());
                log::trace!("Event: new listen address: {address:?}")
            }
            SwarmEvent::ExpiredListenAddr { address, .. } => {
                self.fs_interface
                    .network_interface
                    .listen_addrs
                    .write()
                    .remove(&address);
                log::trace!("Event: expired listen address: {address:?}")
            }
            SwarmEvent::IncomingConnection {
                connection_id,
                send_back_addr,
                ..
            } => {
                if self.closing {
                    log::debug!("Event: Incoming Connection from {send_back_addr:?} while closing: re-closed");
                    self.swarm.close_connection(connection_id);
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                log::debug!(
                    "Event: Connection closed with {peer_id}: {}",
                    cause.unwrap_or(ConnectionError::IO(io::Error::other("no cause given")))
                );
            }
            SwarmEvent::ConnectionEstablished { .. } => {
                // Peer interaction start at identify
            }
            SwarmEvent::NewExternalAddrOfPeer { .. } => {}
            e => log::trace!("Event: {e:?}"),
        };
        false
    }
}
