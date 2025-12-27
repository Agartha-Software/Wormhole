use std::io;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{network::message::Address, pods::{
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
        }
    }

    pub fn get_children(&self) -> io::Result<&Vec<Ino>> {
        match self {
            FsEntry::File(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "entry is not a directory",
            )),
            FsEntry::Directory(children) => Ok(children),
        }
    }
}
