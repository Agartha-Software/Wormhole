use std::fmt::{self, Debug};

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
    RedundancyFile(InodeId, Vec<u8>),
    Rename(InodeId, InodeId, String, String, bool), /// Parent, New Parent, Name, New Name, overwrite
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
            MessageContent::RedundancyFile(_, _) => "RedundancyFile".into(),
            MessageContent::FsAnswer(_, _, _) => "FsAnswer".into(),
            MessageContent::PullAnswer(_, _) => "PullAnswer".into(),
            other => format!("{:?}", other),
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
