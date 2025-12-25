use crate::error::{WhError, WhResult};
use crate::pods::filesystem::fs_interface::FsInterface;
use crate::pods::itree::{ITree, InodeId};
use custom_error::custom_error;

custom_error! {pub GetXAttrError
    WhError{source: WhError} = "{source}",
    KeyNotFound = "Key not found"
}

impl FsInterface {
    pub fn get_inode_xattr(&self, ino: InodeId, key: &str) -> Result<Vec<u8>, GetXAttrError> {
        let itree = ITree::n_read_lock(&self.itree, "fs_interface::get_inode_xattr")?;
        let inode = itree.n_get_inode(ino)?;

        match inode.xattrs.get(key) {
            Some(data) => Ok(data.clone()),
            None => Err(GetXAttrError::KeyNotFound),
        }
    }

    pub fn xattr_exists(&self, ino: InodeId, key: &str) -> WhResult<bool> {
        let itree = ITree::n_read_lock(&self.itree, "fs_interface::xattr_exists")?;
        let inode = itree.n_get_inode(ino)?;

        Ok(inode.xattrs.contains_key(key))
    }

    pub fn list_inode_xattr(&self, ino: InodeId) -> WhResult<Vec<String>> {
        let itree = ITree::n_read_lock(&self.itree, "fs_interface::get_inode_xattr")?;
        let inode = itree.n_get_inode(ino)?;

        Ok(inode.xattrs.keys().cloned().collect())
    }
}
