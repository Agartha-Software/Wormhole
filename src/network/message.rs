use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::{
    config::GlobalConfig,
    pods::{
        filesystem::diffs::{Delta, Signature},
        itree::{ITree, Ino, Inode, Metadata},
        whpath::InodeName,
    },
};

/// Message Content
/// Represent the content of the intern message but is also the struct sent
/// through the network
#[derive(Serialize, Deserialize, Clone)]
pub enum Request {
    Inode(Inode),

    RedundancyFile(Ino, Arc<Vec<u8>>),
    /// Parent, New Parent, Name, New Name, overwrite
    Rename(Ino, Ino, InodeName, InodeName, bool),
    EditHosts(Ino, Vec<PeerId>),
    RevokeFile(Ino, PeerId, Metadata),
    AddHosts(Ino, Vec<PeerId>),
    RemoveHosts(Ino, Vec<PeerId>),

    /// A delta on file write with given base signature
    FileDelta(Ino, Metadata, Signature, Delta),
    /// File contents were changed.
    /// Peers also tracking this file should follow up with a [Request::DeltaRequest]
    FileChanged(Ino, Metadata),

    // RequestFileSignature(Ino),
    // FileSignature(Ino, Vec<u8>),
    RequestFile(Ino),

    Remove(Ino),
    EditMetadata(Ino, Metadata),
    SetXAttr(Ino, String, Vec<u8>),
    RemoveXAttr(Ino, String),

    RequestFs,
    Disconnect,
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Request::Remove(_) => "Remove",
            Request::Inode(_) => "Inode",
            Request::RequestFile(_) => "RequestFile",
            Request::Rename(_, _, _, _, _) => "Rename",
            Request::EditHosts(_, _) => "EditHosts",
            Request::RevokeFile(_, _, _) => "RevokeFile",
            Request::AddHosts(_, _) => "AddHosts",
            Request::RemoveHosts(_, _) => "RemoveHosts",
            Request::EditMetadata(_, _) => "EditMetadata",
            Request::SetXAttr(_, _, _) => "SetXAttr",
            Request::RemoveXAttr(_, _) => "RemoveXAttr",
            Request::RequestFs => "RequestFs",
            Request::RedundancyFile(_, _) => "RedundancyFile",
            Request::Disconnect => "Disconnect",
            Request::FileDelta(_, _, _, _) => "FileDelta",
            Request::FileChanged(_, _) => "FileChanged",
            // Request::RequestFileSignature(_) => "RequestFileSignature",
            // Request::FileSignature(_, _) => "FileSignature",
        };
        write!(f, "{}", name)
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Request::Inode(inode) => write!(
                f,
                "Inode({{{}, name: {}, parent:{}, {}}})",
                inode.id,
                inode.name.as_str(),
                inode.parent,
                match inode.entry {
                    crate::pods::itree::FsEntry::File(_) => 'f',
                    crate::pods::itree::FsEntry::Directory(_) => 'd',
                    crate::pods::itree::FsEntry::Symlink(_) => 'l',
                }
            ),
            Request::RedundancyFile(id, _) => write!(f, "RedundancyFile({id}, <bin>)"),
            Request::Remove(id) => write!(f, "Remove({id})"),
            Request::RequestFile(id) => write!(f, "RequestFile({id})"),
            Request::Rename(parent, new_parent, name, new_name, overwrite) => write!(
                f,
                "Rename(parent: {}, new_parent: {}, name: {}, new_name: {}, overwrite: {})",
                parent,
                new_parent,
                name.as_str(),
                new_name.as_str(),
                overwrite
            ),
            Request::EditHosts(id, hosts) => write!(f, "EditHosts({id}, {hosts:?})"),
            Request::RevokeFile(id, address, _) => {
                write!(f, "RevokeFile({id}, {address}, <metadata>)")
            }
            Request::AddHosts(id, hosts) => write!(f, "AddHosts({id}, {hosts:?})"),
            Request::RemoveHosts(id, hosts) => write!(f, "RemoveHosts({id}, {hosts:?})"),
            Request::EditMetadata(id, metadata) => {
                write!(f, "EditMetadata({id}, {{ perm: {}}})", metadata.perm)
            }
            Request::SetXAttr(id, name, data) => write!(
                f,
                "SetXAttr({id}, {name}, {}",
                String::from_utf8(data.clone()).unwrap_or("<bin>".to_string())
            ),
            Request::RemoveXAttr(id, name) => write!(f, "RemoveXAttr({id}, {name})"),
            Request::RequestFs => write!(f, "RequestFs"),
            Request::Disconnect => write!(f, "Disconnect"),
            Request::FileDelta(ino, meta, _, _) => {
                write!(f, "FileDelta({ino}, {:?})", meta.mtime)
            }
            Request::FileChanged(ino, meta) => {
                write!(f, "FileChanged({ino}, {:?})", meta.mtime)
            } // Request::RequestFileSignature(ino) => write!(f, "RequestFileSignature({ino}, <bin>)"),
              // Request::FileSignature(ino, _) => write!(f, "FileSignature({ino}, <bin>)"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Response {
    /// Request a file delta from this base signature
    DeltaRequest(Ino, Signature),
    // (ITree, peers, global_config)
    FsAnswer(ITree, Vec<PeerId>, GlobalConfig),
    RequestedFile(Vec<u8>),
    Success,
    Failed,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Response::DeltaRequest(_, _) => "DeltaRequest",
            Response::FsAnswer(_, _, _) => "FsAnswer",
            Response::RequestedFile(_) => "RequestedFile",
            Response::Success => "Success!",
            Response::Failed => "Failed...",
        };
        write!(f, "{}", name)
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Response::DeltaRequest(ino, _) => write!(f, "DeltaRequest({ino})"),
            Response::FsAnswer(_, peers, _) => write!(f, "FsAnswer(<bin>, {peers:?}, <bin>"),
            Response::RequestedFile(_) => write!(f, "RequestedFile(<bin>)"),
            Response::Success => write!(f, "Succes!"),
            Response::Failed => write!(f, "Failed..."),
        }
    }
}

/// Message going to the redundancy worker
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RedundancyMessage {
    // PeerSignature(Ino, String, Vec<u8>),
    // WriteDeltas(Ino),
    ApplyTo(Ino),
    CheckIntegrity,
}

/// Message Going To Network
/// Messages sent from fuser to process communicating to the peers
#[derive(Debug)]
pub enum ToNetworkMessage {
    BroadcastMessage(Request),
    SpecificMessage(Request, Vec<PeerId>),
    AnswerMessage(Request, oneshot::Sender<Option<Response>>, PeerId),
    CloseNetwork,
}

impl fmt::Display for ToNetworkMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ToNetworkMessage::BroadcastMessage(content) => {
                write!(f, "ToNetworkMessage::BroadcastMessage({})", content)
            }
            ToNetworkMessage::SpecificMessage(content, peer) => {
                write!(
                    f,
                    "ToNetworkMessage::SpecificMessage({}, to: {:?})",
                    content, peer
                )
            }
            ToNetworkMessage::AnswerMessage(content, _, peer) => {
                write!(
                    f,
                    "ToNetworkMessage::AnswerMessage({}, to: {:?}) with callback",
                    content, peer
                )
            }
            ToNetworkMessage::CloseNetwork => write!(f, "ToNetworkMessage::CloseNetwork"),
        }
    }
}
