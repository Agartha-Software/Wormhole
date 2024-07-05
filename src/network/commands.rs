use std::sync::Arc;

use super::{message::{self, NetworkMessage}, peer_ipc::PeerIPC, server::State};


pub async fn aquire(
    state: &Arc<State>,
    pod: &String,
    file_path: &String,
) {

}

pub async fn release(
    state: &Arc<State>,
    pod: &String,
    file_path: &String,
) {

}
pub async fn publish_file<'a>(
    state: &'a Arc<State>,
    pod: &String,
    file_path: &String,
    peer: &String
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let pods = state.pods.read()?;
    let thispod = pods
    .get(pod).ok_or(std::io::Error::other("pod not registered"))?;
    let file = std::fs::read(thispod.path.join(file_path))?;
    let message = NetworkMessage::File(message::File {pod: pod.clone(), path: file_path.clone(), file});
    let lock = state.peers.read()?;
    if let Some(found) = lock.iter().find(|p| p.address == *peer) {
        found.sender.send(message.clone())?;
    } else {
        drop(lock);
        let mut lock = state.peers.write()?;
        let peer_ipc = PeerIPC::connect(peer.clone(), state);
        peer_ipc.sender.send(message.clone())?;
        lock.push(peer_ipc);
    }
    Ok(())
}

pub async fn publish_meta<'a>(
    state: &'a Arc<State>,
    pod: &String,
    file_path: &String,
) -> Result<(), Box<dyn std::error::Error + 'a>> {
    let pods = state.pods.read()?;
    let thispod = pods.get(pod).ok_or(std::io::Error::other("pod not registered"))?;
    let nw = &thispod
        .network;
    let meta = NetworkMessage::Meta(thispod.metas.get(file_path).map(|m|m.clone()).ok_or(std::io::Error::other("file not found"))?);
    for peer in &nw.peers {
        let lock = state.peers.read()?;
        if let Some(found) = lock.iter().find(|p| p.address == *peer) {
            found.sender.send(meta.clone());
        } else {
            drop(lock);
            let mut lock = state.peers.write()?;
            let peer_ipc = PeerIPC::connect(peer.clone(), state);
            peer_ipc.sender.send(meta.clone());
            lock.push(peer_ipc);
        }
    }
    Ok(())
}

pub async fn get_file<'a>(
    state: &'a Arc<State>,
    pod: &String,
    file_path: &String,
) -> Result<Vec<u8>, Box<dyn std::error::Error + 'a>> {
    let pods = state.pods.read()?;
    let thispod = pods.get(pod).ok_or(std::io::Error::other("pod not registered"))?;
    let nw = &thispod.network;
    // let file = std::fs::read(file_path)?;
    let file = thispod.directory.open_file(file_path);
    let request = NetworkMessage::RequestFile(message::Path{pod: pod.clone(), file: file_path.clone()});
    if let Some(peer) = nw.peers.first() {
        let lock = state.peers.read()?;
        let mut receiver = match lock.iter().find(|p| p.address == *peer) {
            Some(found) => {
                let receiver = found.receiver.resubscribe();
                found.sender.send(request.clone());
                drop(lock);
                receiver
            }
            _ => {
                drop(lock);
                let mut lock = state.peers.write()?;
                let peer_ipc = PeerIPC::connect(peer.clone(), state);
                let receiver = peer_ipc.receiver.resubscribe();
                peer_ipc.sender.send(request.clone());
                lock.push(peer_ipc);
                receiver
            }
        };
        while let Ok(message) = receiver.recv().await {
            if let NetworkMessage::File(message::File{pod, path, file}) = message {
                return Ok(file);
            }
        }
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "no response from peer")));
    }
    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "no peer to get from")));
}
