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

/// Network message content for distribution and synchronization filesystem operations.
/// Represent both internal messages and data structures sent over the network between peers.
/// Each variant corresponds to a specific filesystem operation or request.
#[derive(Serialize, Deserialize, Clone)]
pub enum MessageContent {
    /// Remove an inode from the filesystem.
    /// Contains (file_id).
    Remove(InodeId),

    /// Add or update an inode in the filesystem.
    /// Contains (file_id).
    Inode(Inode),

    /// Request file data for a specific inode from a peer.
    /// Contains (file_id, requesting_peer_address).
    RequestFile(InodeId, Address),

    /// Response to RequestFile containing the file data.
    /// Contains (file_id, file_data).
    PullAnswer(InodeId, Vec<u8>),

    /// Send a redundancy chunk of a file to a peer.
    /// Contains (file_id, redundancy_data) with Arc for thread-safe.
    RedundancyFile(InodeId, Arc<Vec<u8>>),

    /// Rename or move an inode within the filesystem.
    /// Contains (current_parent_id, new_parent_id, current_name, new_name, overwrite
    Rename(InodeId, InodeId, String, String, bool),

    /// Update the list of hosts for a file.
    /// Contains (file_id, new_hosts).
    EditHosts(InodeId, Vec<Address>),

    /// Revoke file from remote hosts after local modification.
    /// Contains (file_id, revoking_address, update_metadata).
    RevokeFile(InodeId, Address, Metadata),

    /// Add new hosts to a file's host list.
    /// Contains (file_id, new_hosts).
    AddHosts(InodeId, Vec<Address>),

    /// Remove hosts from a file's host list.
    /// Contains (file_id, hosts_to_remove).
    RemoveHosts(InodeId, Vec<Address>),

    /// Update an inode's metadata (permissions, timestamps, etc.).
    /// Contains (file_id, new_metadata).
    EditMetadata(InodeId, Metadata),

    /// Set extended attribute for an inode.
    /// Contains (file_id, attribute_name, attribute_data).
    SetXAttr(InodeId, String, Vec<u8>),

    /// Remove extended attribute from an inode.
    /// Contains (file_id, attribute_name).
    RemoveXAttr(InodeId, String),

    /// Request the full filesystem structure from a peer.
    /// Used for initial sync when joining the network.
    RequestFs,

    /// Notify peers of disconnection.
    /// Contains (disconnecting_peer_address).
    Disconnect(Address),

    /// Response to RequestFs containing the full filesystem structure.
    /// Contains (serialized_filesystem, list_of_peers, optional_binary_data).
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

/// Message content paired with an optional status callback.
///
/// Used for tracking message delivery status. The sender can provide
/// a callback channel to receive confirmation when the message is processed.
pub type MessageAndStatus = (MessageContent, Option<UnboundedSender<WhResult<()>>>);

/// Network address representation for peer identification.
///
/// Simple string-based address format (e.g., "IP:Port")
/// used to identify and communicate with peers in the network.
pub type Address = String;

/// Incoming message from network peers.
///
/// Wraps a message with its origin information for processing by the network message handler.
/// Contains both the sender's address and the message content.
/// Messages received by peers, forwared to [crate::network::watchdogs::network_file_actions]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FromNetworkMessage {
    /// Adress of the peer that sent this message
    pub origin: Address,

    /// The actual message content to be processed
    pub content: MessageContent,
}

/// Commands for the redundancy management worker.
///
/// Controls redundancy operations to ensure files are replicated across multiple peers for fault tolerance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RedundancyMessage {
    /// Apply redundancy to a specific inode.
    /// Ensures the file is replicated to the required number of peers.
    /// Contains (file_id).
    ApplyTo(InodeId),

    /// Check and fix redundancy for across all files.
    /// Scans the filesystem and ensures all files meet redundancy requirements.
    CheckIntegrity,
}

/// Message Going To Network
/// Messages sent from fuser to process communicating to the peers
#[derive(Debug)]
pub enum ToNetworkMessage {
    /// Broadcast message to all connected peers.
    /// Contains (message_content).
    BroadcastMessage(MessageContent),
    /// Send a message to specific peers.
    /// Contains (message_and_status, list_of_target_addresses).
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

/// Serialized filesystem state for network transmission.
/// Used to send the entire filesystem structure to a peer during initial sync.
/// Contains the filesystem index and the next available inode ID.
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileSystemSerialized {
    /// The complete filesystem tree index.
    pub fs_index: ArboIndex,
    /// The next available inode ID for creating new files/directories.
    pub next_inode: InodeId,
}
