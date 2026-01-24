use crate::{
    error::{WhError, WhResult},
    network::message::{Request, Response, ToNetworkMessage},
    pods::{
        itree::{ITree, Ino},
        network::network_interface::NetworkInterface,
    },
};

impl NetworkInterface {
    pub fn set_inode_xattr(&self, ino: Ino, key: &str, data: Vec<u8>) -> WhResult<()> {
        ITree::write_lock(&self.itree, "network_interface::get_inode_xattr")?.set_inode_xattr(
            ino,
            key,
            data.clone(),
        )?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(Request::SetXAttr(
                ino,
                key.to_owned(),
                data,
            )))
            .or(Err(WhError::NetworkDied {
                called_from: "set_inode_xattr".to_string(),
            }))
    }

    pub fn recept_inode_xattr(&self, ino: Ino, key: &str, data: Vec<u8>) -> WhResult<Response> {
        ITree::write_lock(&self.itree, "network_interface::get_inode_xattr")?
            .set_inode_xattr(ino, key, data)?;
        Ok(Response::Success)
    }

    pub fn remove_inode_xattr(&self, ino: Ino, key: &str) -> WhResult<()> {
        ITree::write_lock(&self.itree, "network_interface::get_inode_xattr")?
            .remove_inode_xattr(ino, key)?;

        self.to_network_message_tx
            .send(ToNetworkMessage::BroadcastMessage(Request::RemoveXAttr(
                ino,
                key.to_owned(),
            )))
            .or(Err(WhError::NetworkDied {
                called_from: "set_inode_xattr".to_string(),
            }))
    }

    pub fn recept_remove_inode_xattr(&self, ino: Ino, key: &str) -> WhResult<Response> {
        ITree::write_lock(&self.itree, "network_interface::get_inode_xattr")?
            .remove_inode_xattr(ino, key)?;
        Ok(Response::Success)
    }
}
