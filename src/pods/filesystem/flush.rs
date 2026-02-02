use crate::{
    error::WhError,
    network::message::{Request, Response, ToNetworkMessage},
    pods::{
        filesystem::{
            attrs::AcknoledgeSetAttrError,
            diffs::{Delta, DiffError, Dlt, Sig, Signature},
            file_handle::FileHandle,
            fs_interface::FsInterface,
            read::ReadError,
            write::WriteError,
        },
        itree::{FsEntry, ITree, Ino, Metadata},
        network::pull_file::PullError,
    },
};
use custom_error::custom_error;
use libp2p::PeerId;

custom_error! {
    #[derive(Clone)]
    pub FlushError
    WhError{source: WhError} = "{source}",
    ReadError{source: ReadError} = "{source}",
    WriteError{source: WriteError} = "{source}",
    DiffError{source: DiffError} = "{source}",
    PullError{source: PullError} = "{source}",
}

impl FsInterface {
    /// Flush file changes to all connected peers
    /// With a handle, the changed will be first attempted with the handle's base signature
    /// the handle's signature will be flushed. The dirty flag will be reset
    /// Without a handle, all peers will get a 'FileChanged' message and tracking peers will request the delta with their own base signature
    /// Non-tracking peers will always only get a 'FileChanged' message
    ///
    pub fn flush(&self, ino: Ino, handle: Option<&mut FileHandle>) -> Result<(), FlushError> {
        let peers_info = self.network_interface.peers_info.read();
        let peers = peers_info.keys();
        let inode = self.network_interface.itree.read().get_inode(ino)?.clone();
        let tracking = match &inode.entry {
            FsEntry::File(tracking) => tracking,
            _ => return Err(WhError::InodeIsADirectory.into()),
        };
        if let Some((signature, dirty)) =
            handle.and_then(|h| h.signature.as_mut().map(|s| (s, &mut h.dirty)))
        {
            if !*dirty {
                return Ok(());
            }
            let file = self.get_whole_file_sync(ino)?;
            let delta = signature.diff(&file)?;

            let old_sig = signature.clone();
            *signature = Signature::new(&file)?;
            *dirty = false;

            for peer in peers {
                if tracking.contains(peer) {
                    self.network_interface
                        .to_network_message_tx
                        .send(ToNetworkMessage::SpecificMessage(
                            Request::FileDelta(
                                ino,
                                inode.meta.clone(),
                                old_sig.clone(),
                                delta.clone(),
                            ),
                            vec![*peer],
                        ))
                        .map_err(|e| WhError::WouldBlock {
                            called_from: e.to_string(),
                        })?;
                } else {
                    self.network_interface
                        .to_network_message_tx
                        .send(ToNetworkMessage::SpecificMessage(
                            Request::FileChanged(ino, inode.meta.clone()),
                            vec![*peer],
                        ))
                        .map_err(|e| WhError::WouldBlock {
                            called_from: e.to_string(),
                        })?;
                }
            }
        } else {
            self.network_interface
                .to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(Request::FileChanged(
                    ino,
                    inode.meta.clone(),
                )))
                .map_err(|e| WhError::WouldBlock {
                    called_from: e.to_string(),
                })?;
        }
        Ok(())
    }

    /// Apply a delta received from the network
    /// deltas are in reference to a base signature, in case of signature mismatch
    /// [Request::DeltaRequest] is emitted back to get the correct diff
    ///
    pub fn accept_delta(
        &self,
        ino: Ino,
        meta: Metadata,
        sig: Signature,
        delta: Delta,
    ) -> Result<Response, FlushError> {
        log::trace!("accept_delta({ino})");
        let file = match self.get_local_file(ino)? {
            Some(file) => file,
            None => {
                log::warn!("accept_delta: received delta but isn't currently tracking the file!");
                return Ok(Response::Success);
            }
        };
        let local_sig = Signature::new_using(&file, sig.implementor())?;
        log::trace!(
            "signing <<\n{}\n>> = {:?}",
            file.0.escape_ascii(),
            local_sig
        );
        if local_sig == sig {
            let patched = delta.patch(&file)?;
            log::trace!(
                "accept_delta: patched = {}",
                String::from_utf8_lossy(&patched.0)
            );

            let itree = ITree::read_lock(&self.network_interface.itree, "fs_interface.write")?;
            let path = itree.get_path_from_inode_id(ino)?;
            drop(itree);

            self.disk
                .write_file(&path, &patched.0, 0)
                .map_err(WriteError::from)?;
            self.acknowledge_metadata(ino, meta).map_err(|e| match e {
                AcknoledgeSetAttrError::WhError { source } => FlushError::from(source),
                AcknoledgeSetAttrError::SetFileSizeIoError { io } => WriteError::from(io).into(),
            })?;
        } else {
            log::warn!("accept_delta: signature does not match local sig!");
            return Ok(Response::DeltaRequest(ino, local_sig));
        }
        Ok(Response::Success)
    }

    /// Acknowledge a file change and request the change contents if we are tracking the file
    pub fn accept_file_changed(&self, ino: Ino, meta: Metadata) -> Result<Response, FlushError> {
        self.acknowledge_metadata(ino, meta).map_err(|e| match e {
            AcknoledgeSetAttrError::WhError { source } => FlushError::from(source),
            AcknoledgeSetAttrError::SetFileSizeIoError { io } => WriteError::from(io).into(),
        })?;
        let file = match self.get_local_file(ino)? {
            Some(file) => file,
            None => return Ok(Response::Success),
        };
        let local_sig = Signature::new(&file)?;
        Ok(Response::DeltaRequest(ino, local_sig))
    }

    pub fn respond_delta(
        &self,
        ino: Ino,
        sig: Signature,
        origin: PeerId,
    ) -> Result<(), FlushError> {
        let file = self.get_local_file(ino)?.ok_or(WhError::InodeNotFound)?;
        let delta = sig.diff(&file)?;

        let inode = self.network_interface.itree.read().get_inode(ino)?.clone();

        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                Request::FileDelta(ino, inode.meta, sig, delta),
                vec![origin],
            ))
            .map_err(|e| WhError::WouldBlock {
                called_from: e.to_string(),
            })?;
        Ok(())
    }
}
