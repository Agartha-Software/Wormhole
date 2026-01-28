use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    error::WhResult,
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
pub enum MessageContent {
    Inode(Inode),

    RedundancyFile(Ino, Arc<Vec<u8>>),
    /// Parent, New Parent, Name, New Name, overwrite
    Rename(Ino, Ino, InodeName, InodeName, bool),
    EditHosts(Ino, Vec<Address>),
    RevokeFile(Ino, Address, Metadata),
    AddHosts(Ino, Vec<Address>),
    RemoveHosts(Ino, Vec<Address>),

    /// A delta on file write with given base signature
    FileDelta(Ino, Metadata, Signature, Delta),
    /// File contents were changed.
    /// Peers also tracking this file should follow up with a [MessageContent::DeltaRequest]
    FileChanged(Ino, Metadata),
    /// Request a file delta from this base signature
    DeltaRequest(Ino, Signature),

    // RequestFileSignature(Ino),
    // FileSignature(Ino, Vec<u8>),
    RequestFile(Ino),
    PullAnswer(Ino, Vec<u8>),

    Remove(Ino),
    EditMetadata(Ino, Metadata),
    SetXAttr(Ino, String, Vec<u8>),
    RemoveXAttr(Ino, String),

    RequestFs,
    // (ITree, peers, global_config)
    FsAnswer(ITree, Vec<Address>, Vec<u8>),

    Disconnect,
}

impl fmt::Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            MessageContent::Remove(_) => "Remove",
            MessageContent::Inode(_) => "Inode",
            MessageContent::RequestFile(_) => "RequestFile",
            MessageContent::PullAnswer(_, _) => "PullAnswer",
            MessageContent::Rename(_, _, _, _, _) => "Rename",
            MessageContent::EditHosts(_, _) => "EditHosts",
            MessageContent::RevokeFile(_, _, _) => "RevokeFile",
            MessageContent::AddHosts(_, _) => "AddHosts",
            MessageContent::RemoveHosts(_, _) => "RemoveHosts",
            MessageContent::EditMetadata(_, _) => "EditMetadata",
            MessageContent::SetXAttr(_, _, _) => "SetXAttr",
            MessageContent::RemoveXAttr(_, _) => "RemoveXAttr",
            MessageContent::RequestFs => "RequestFs",
            MessageContent::FsAnswer(_, _, _) => "FsAnswer",
            MessageContent::RedundancyFile(_, _) => "RedundancyFile",
            MessageContent::Disconnect => "Disconnect",
            MessageContent::FileDelta(_, _, _, _) => "FileDelta",
            MessageContent::FileChanged(_, _) => "FileChanged",
            MessageContent::DeltaRequest(_, _) => "DeltaRequest",
            // MessageContent::RequestFileSignature(_) => "RequestFileSignature",
            // MessageContent::FileSignature(_, _) => "FileSignature",
        };
        write!(f, "{}", name)
    }
}

impl fmt::Debug for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageContent::Inode(inode) => write!(
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
            MessageContent::RedundancyFile(id, _) => write!(f, "RedundancyFile({id}, <bin>)"),
            MessageContent::FsAnswer(_, peers, _) => write!(f, "FsAnswer(<bin>, {peers:?}, <bin>"),
            MessageContent::PullAnswer(id, _) => write!(f, "PullAnswer({id}, <bin>)"),
            MessageContent::Remove(id) => write!(f, "Remove({id})"),
            MessageContent::RequestFile(id) => write!(f, "RequestFile({id})"),
            MessageContent::Rename(parent, new_parent, name, new_name, overwrite) => write!(
                f,
                "Rename(parent: {}, new_parent: {}, name: {}, new_name: {}, overwrite: {})",
                parent,
                new_parent,
                name.as_str(),
                new_name.as_str(),
                overwrite
            ),
            MessageContent::EditHosts(id, hosts) => write!(f, "EditHosts({id}, {hosts:?})"),
            MessageContent::RevokeFile(id, address, _) => {
                write!(f, "RevokeFile({id}, {address}, <metadata>)")
            }
            MessageContent::AddHosts(id, hosts) => write!(f, "AddHosts({id}, {hosts:?})"),
            MessageContent::RemoveHosts(id, hosts) => write!(f, "RemoveHosts({id}, {hosts:?})"),
            MessageContent::EditMetadata(id, metadata) => {
                write!(f, "EditMetadata({id}, {{ perm: {}}})", metadata.perm)
            }
            MessageContent::SetXAttr(id, name, data) => write!(
                f,
                "SetXAttr({id}, {name}, {}",
                String::from_utf8(data.clone()).unwrap_or("<bin>".to_string())
            ),
            MessageContent::RemoveXAttr(id, name) => write!(f, "RemoveXAttr({id}, {name})"),
            MessageContent::RequestFs => write!(f, "RequestFs"),
            MessageContent::Disconnect => write!(f, "Disconnect"),
            MessageContent::FileDelta(ino, meta, _, _) => {
                write!(f, "FileDelta({ino}, {:?})", meta.mtime)
            }
            MessageContent::FileChanged(ino, meta) => {
                write!(f, "FileChanged({ino}, {:?})", meta.mtime)
            }
            MessageContent::DeltaRequest(ino, _) => write!(f, "DeltaRequest({ino})"),
            // MessageContent::RequestFileSignature(ino) => write!(f, "RequestFileSignature({ino}, <bin>)"),
            // MessageContent::FileSignature(ino, _) => write!(f, "FileSignature({ino}, <bin>)"),
        }
    }
}

pub type MessageAndStatus = (MessageContent, Option<UnboundedSender<WhResult<()>>>);

pub type Address = String;

/// Message Coming from Network
/// Messages recived by peers, forwared to [crate::network::watchdogs::network_file_actions]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FromNetworkMessage {
    pub origin: Address,
    pub content: MessageContent,
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
    BroadcastMessage(MessageContent),
    SpecificMessage(MessageAndStatus, Vec<Address>),
}

impl fmt::Display for ToNetworkMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ToNetworkMessage::BroadcastMessage(content) => {
                write!(f, "ToNetworkMessage::BroadcastMessage({})", content)
            }
            ToNetworkMessage::SpecificMessage((content, callback), adress) => {
                write!(
                    f,
                    "ToNetworkMessage::SpecificMessage({}, callback: {}, to: {:?})",
                    content,
                    callback.is_some(),
                    adress
                )
            }
        }
    }
}
