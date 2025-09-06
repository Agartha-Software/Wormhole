use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    error::WhResult,
    pods::arbo::{ArboIndex, Inode, InodeId, Metadata},
};

/// Message Content
/// Represent the content of the intern message but is also the struct sent
/// through the network
#[derive(Serialize, Deserialize, Clone)]
pub enum MessageContent {
    Remove(InodeId),
    Inode(Inode),
    RequestFile(InodeId, Address),
    PullAnswer(InodeId, Vec<u8>),
    RedundancyFile(InodeId, Arc<Vec<u8>>),
    /// Parent, New Parent, Name, New Name, overwrite
    Rename(InodeId, InodeId, String, String, bool),
    EditHosts(InodeId, Vec<Address>),
    RevokeFile(InodeId, Address, Metadata),
    AddHosts(InodeId, Vec<Address>),
    RemoveHosts(InodeId, Vec<Address>),
    EditMetadata(InodeId, Metadata),
    SetXAttr(InodeId, String, Vec<u8>),
    RemoveXAttr(InodeId, String),
    RequestFs,
    Disconnect(Address),

    // (Arbo, peers, global_config)
    FsAnswer(FileSystemSerialized, Vec<Address>, Vec<u8>),
}

impl fmt::Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            MessageContent::Remove(_) => "Remove",
            MessageContent::Inode(_) => "Inode",
            MessageContent::RequestFile(_, _) => "RequestFile",
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
            MessageContent::Disconnect(_) => "Disconnect",
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
                inode.name,
                inode.parent,
                match inode.entry {
                    crate::pods::arbo::FsEntry::File(_) => 'f',
                    crate::pods::arbo::FsEntry::Directory(_) => 'd',
                }
            ),
            MessageContent::RedundancyFile(id, _) => write!(f, "RedundancyFile({id}, <bin>)"),
            MessageContent::FsAnswer(_, peers, _) => write!(f, "FsAnswer(<bin>, {peers:?}, <bin>"),
            MessageContent::PullAnswer(id, _) => write!(f, "PullAnswer({id}, <bin>)"),
            MessageContent::Remove(id) => write!(f, "Remove({id})"),
            MessageContent::RequestFile(id, y) => write!(f, "RequestFile({id}, {y})"),
            MessageContent::Rename(parent, new_parent, name, new_name, overwrite) => write!(
                f,
                "Rename(parent: {}, new_parent: {}, name: {}, new_name: {}, overwrite: {})",
                parent, new_parent, name, new_name, overwrite
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
            MessageContent::Disconnect(address) => write!(f, "Disconnect({address})"),
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
    ApplyTo(InodeId),
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

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileSystemSerialized {
    pub fs_index: ArboIndex,
    pub next_inode: InodeId,
}
