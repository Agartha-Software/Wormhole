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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageContent {
    Register(Address),
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
    // Arbo, peers, .global_config
    FsAnswer(FileSystemSerialized, Vec<Address>, Vec<u8>),
}

impl fmt::Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            MessageContent::Register(_) => "Register",
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
