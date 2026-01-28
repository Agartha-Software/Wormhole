pub mod attrs;
pub mod diffs;
pub mod file_handle;
pub mod flush;
pub mod fs_interface;
pub mod make_inode;
pub mod open;
pub mod permissions;
pub mod read;
pub mod readdir;
pub mod release;
pub mod remove_inode;
pub mod rename;
pub mod write;
pub mod xattrs;

use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct File(pub Arc<Vec<u8>>);

impl File {
    pub fn empty() -> Self {
        Self(Arc::new(Vec::new()))
    }
}

impl Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.escape_ascii().to_string())
    }
}
