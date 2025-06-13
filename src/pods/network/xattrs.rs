use crate::{
    error::{WhError, WhResult},
    network::message::{MessageContent, ToNetworkMessage},
    pods::{
        arbo::{Arbo, InodeId},
        network::network_interface::NetworkInterface,
    },
};

impl NetworkInterface {
    pub fn set_inode_xattr(&self, ino: InodeId, key: String, data: Vec<u8>) -> WhResult<()> {
        Arbo::n_write_lock(&self.arbo, "network_interface::get_inode_xattr")?.set_inode_xattr(
            ino,
            key.clone(),
            data.clone(),
        )?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::SetXAttr(ino, key, data),
            ))
            .or(Err(WhError::NetworkDied {
                called_from: "set_inode_xattr".to_string(),
            }))
    }

    pub fn recept_inode_xattr(&self, ino: InodeId, key: String, data: Vec<u8>) -> WhResult<()> {
        Arbo::n_write_lock(&self.arbo, "network_interface::get_inode_xattr")?.set_inode_xattr(
            ino,
            key.clone(),
            data,
        )
    }

    pub fn remove_inode_xattr(&self, ino: InodeId, key: String) -> WhResult<()> {
        Arbo::n_write_lock(&self.arbo, "network_interface::get_inode_xattr")?
            .remove_inode_xattr(ino, key.clone())?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::RemoveXAttr(ino, key),
            ))
            .or(Err(WhError::NetworkDied {
                called_from: "set_inode_xattr".to_string(),
            }))
    }

    pub fn recept_remove_inode_xattr(&self, ino: InodeId, key: String) -> WhResult<()> {
        Arbo::n_write_lock(&self.arbo, "network_interface::get_inode_xattr")?
            .remove_inode_xattr(ino, key.clone())
    }
}
