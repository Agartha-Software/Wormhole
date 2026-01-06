use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{error::{WhError, WhResult}, network::message::Address, pods::{
    filesystem::fs_interface::SimpleFileType,
    itree::Ino, whpath::WhPath,
}};

pub type Hosts = Vec<Address>;


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SymlinkPath {
    /// Path relative to the symlink file itself
    SymlinkPathRelative(Utf8PathBuf),
    /// Path relative to the WH drive. Not really absolute but emulates absolute symlinks within the WH drive
    SymlinkPathAbsolute(WhPath),
    /// absolute Path pointing outside the WH drive
    SymlinkExternal(Utf8PathBuf),
}


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EntrySymlink {
    pub target: SymlinkPath,
}

impl Default for EntrySymlink {
    fn default() -> Self {
        Self {
            target: SymlinkPath::SymlinkPathRelative("".into())
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
/// Should be extended until meeting [fuser::FileType]
pub enum FsEntry {
    File(Hosts),
    Directory(Vec<Ino>),
    Symlink(EntrySymlink)
}

impl FsEntry {
    pub fn get_filetype(&self) -> SimpleFileType {
        match self {
            FsEntry::File(_) => SimpleFileType::File,
            FsEntry::Directory(_) => SimpleFileType::Directory,
            FsEntry::Symlink(_) => SimpleFileType::Symlink,
        }
    }

    pub fn get_children(&self) -> WhResult<&Vec<Ino>> {
        match self {
            FsEntry::Directory(children) => Ok(children),
            _ => Err(WhError::InodeIsNotADirectory),
        }
    }
}
