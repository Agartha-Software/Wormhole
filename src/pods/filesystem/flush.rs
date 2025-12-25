use crate::{
    error::WhError,
    network::message::{Address, MessageContent, ToNetworkMessage},
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
        let peers = self
            .network_interface
            .peers
            .read()
            .iter()
            .map(|p| p.hostname.clone())
            .collect::<Vec<_>>();
        let itree = self.itree.read();
        let inode = itree.n_get_inode(ino)?.clone();
        drop(itree);
        let tracking = match &inode.entry {
            FsEntry::File(tracking) => tracking,
            FsEntry::Directory(_) => return Err(WhError::InodeIsADirectory.into()),
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

            for peer in &peers {
                if tracking.contains(peer) {
                    self.network_interface
                        .to_network_message_tx
                        .send(ToNetworkMessage::SpecificMessage(
                            (
                                MessageContent::FileDelta(
                                    ino,
                                    inode.meta.clone(),
                                    old_sig.clone(),
                                    delta.clone(),
                                ),
                                None,
                            ),
                            vec![peer.clone()],
                        ))
                        .map_err(|e| WhError::WouldBlock {
                            called_from: e.to_string(),
                        })?;
                } else {
                    self.network_interface
                        .to_network_message_tx
                        .send(ToNetworkMessage::SpecificMessage(
                            (MessageContent::FileChanged(ino, inode.meta.clone()), None),
                            vec![peer.clone()],
                        ))
                        .map_err(|e| WhError::WouldBlock {
                            called_from: e.to_string(),
                        })?;
                }
            }
        } else {
            self.network_interface
                .to_network_message_tx
                .send(ToNetworkMessage::BroadcastMessage(
                    MessageContent::FileChanged(ino, inode.meta.clone()),
                ))
                .map_err(|e| WhError::WouldBlock {
                    called_from: e.to_string(),
                })?;
        }
        Ok(())
    }

    /// Apply a delta received from the network
    /// deltas are in reference to a base signature, in case of signature mismatch
    /// [MessageContent::DeltaRequest] is emitted back to get the correct diff
    ///
    pub fn accept_delta(
        &self,
        ino: Ino,
        meta: Metadata,
        sig: Signature,
        delta: Delta,
        origin: Address,
    ) -> Result<(), FlushError> {
        log::trace!("accept_delta({ino})");
        let file = match self.get_local_file(ino)? {
            Some(file) => file,
            None => {
                log::warn!("accept_delta: received delta but isn't currently tracking the file!");
                return Ok(());
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

            let itree = ITree::n_read_lock(&self.itree, "fs_interface.write")?;
            let path = itree.n_get_path_from_inode_id(ino)?;
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
            self.network_interface
                .to_network_message_tx
                .send(ToNetworkMessage::SpecificMessage(
                    (MessageContent::DeltaRequest(ino, local_sig), None),
                    vec![origin],
                ))
                .expect("pull_file: unable to request on the network thread");
        }
        Ok(())
    }

    /// Acknowledge a file change and request the change contents if we are tracking the file
    pub fn accept_file_changed(
        &self,
        ino: Ino,
        meta: Metadata,
        origin: Address,
    ) -> Result<(), FlushError> {
        self.acknowledge_metadata(ino, meta).map_err(|e| match e {
            AcknoledgeSetAttrError::WhError { source } => FlushError::from(source),
            AcknoledgeSetAttrError::SetFileSizeIoError { io } => WriteError::from(io).into(),
        })?;
        let file = match self.get_local_file(ino)? {
            Some(file) => file,
            None => return Ok(()),
        };
        let local_sig = Signature::new(&file)?;
        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                (MessageContent::DeltaRequest(ino, local_sig), None),
                vec![origin],
            ))
            .expect("pull_file: unable to request on the network thread");
        Ok(())
    }

    pub fn respond_delta(
        &self,
        ino: Ino,
        sig: Signature,
        origin: Address,
    ) -> Result<(), FlushError> {
        let file = self.get_local_file(ino)?.ok_or(WhError::InodeNotFound)?;
        let delta = sig.diff(&file)?;

        let itree = self.itree.read();
        let inode = itree.n_get_inode(ino)?.clone();
        drop(itree);

        self.network_interface
            .to_network_message_tx
            .send(ToNetworkMessage::SpecificMessage(
                (MessageContent::FileDelta(ino, inode.meta, sig, delta), None),
                vec![origin],
            ))
            .map_err(|e| WhError::WouldBlock {
                called_from: e.to_string(),
            })?;
        Ok(())
    }
}
