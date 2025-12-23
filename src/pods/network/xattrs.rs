use crate::{
    error::{WhError, WhResult},
    network::message::{MessageContent, ToNetworkMessage},
    pods::{
        itree::{ITree, InodeId},
        network::network_interface::NetworkInterface,
    },
};

impl NetworkInterface {
    pub fn set_inode_xattr(&self, ino: InodeId, key: &str, data: Vec<u8>) -> WhResult<()> {
        ITree::n_write_lock(&self.itree, "network_interface::get_inode_xattr")?.set_inode_xattr(
            ino,
            key,
            data.clone(),
        )?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::SetXAttr(ino, key.to_owned(), data),
            ))
            .or(Err(WhError::NetworkDied {
                called_from: "set_inode_xattr".to_string(),
            }))
    }

    pub fn recept_inode_xattr(&self, ino: InodeId, key: &str, data: Vec<u8>) -> WhResult<()> {
        ITree::n_write_lock(&self.itree, "network_interface::get_inode_xattr")?
            .set_inode_xattr(ino, key, data)
    }

    pub fn remove_inode_xattr(&self, ino: InodeId, key: &str) -> WhResult<()> {
        ITree::n_write_lock(&self.itree, "network_interface::get_inode_xattr")?
            .remove_inode_xattr(ino, key)?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::RemoveXAttr(ino, key.to_owned()),
            ))
            .or(Err(WhError::NetworkDied {
                called_from: "set_inode_xattr".to_string(),
            }))
    }

    pub fn recept_remove_inode_xattr(&self, ino: InodeId, key: &str) -> WhResult<()> {
        ITree::n_write_lock(&self.itree, "network_interface::get_inode_xattr")?
            .remove_inode_xattr(ino, key)
    }
}
