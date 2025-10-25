use std::ffi::{OsStr, OsString};

use crate::error::{WhError, WhResult};
use crate::pods::arbo::{Arbo, InodeId};
use crate::pods::filesystem::fs_interface::FsInterface;
use custom_error::custom_error;

custom_error! {pub GetXAttrError
    WhError{source: WhError} = "{source}",
    KeyNotFound = "Key not found"
}

impl FsInterface {
    pub fn get_inode_xattr(&self, ino: InodeId, key: &OsStr) -> Result<Vec<u8>, GetXAttrError> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::get_inode_xattr")?;
        let inode = arbo.n_get_inode(ino)?;

        match inode.xattrs.get(key) {
            Some(data) => Ok(data.clone()),
            None => Err(GetXAttrError::KeyNotFound),
        }
    }

    pub fn xattr_exists(&self, ino: InodeId, key: &OsStr) -> WhResult<bool> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::xattr_exists")?;
        let inode = arbo.n_get_inode(ino)?;

        Ok(inode.xattrs.contains_key(key))
    }

    pub fn list_inode_xattr(&self, ino: InodeId) -> WhResult<Vec<OsString>> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::get_inode_xattr")?;
        let inode = arbo.n_get_inode(ino)?;

        Ok(inode.xattrs.keys().cloned().collect())
    }
}
