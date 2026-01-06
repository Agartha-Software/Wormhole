use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::{WhError, WhResult},
    network::message::Address,
    pods::{filesystem::fs_interface::SimpleFileType, itree::Ino, whpath::WhPath},
};

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

impl SymlinkPath {
    pub fn realize(&self, from: &Path) -> PathBuf {
        return match self {
            SymlinkPath::SymlinkPathRelative(path) => path.into(),
            SymlinkPath::SymlinkPathAbsolute(path) => from.join(path),
            SymlinkPath::SymlinkExternal(path) => path.into(),
        };
    }
}

impl Display for SymlinkPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymlinkPath::SymlinkPathRelative(path) => f.write_str(path.as_str()),
            SymlinkPath::SymlinkPathAbsolute(path) => write!(f, "//{}", path.as_str()),
            SymlinkPath::SymlinkExternal(path) => f.write_str(path.as_str()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EntrySymlink {
    pub target: SymlinkPath,
    pub hint: Option<SimpleFileType>,
}

impl Default for EntrySymlink {
    fn default() -> Self {
        Self {
            target: SymlinkPath::SymlinkPathRelative(".".into()),
            hint: Some(SimpleFileType::Directory),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
/// Should be extended until meeting [fuser::FileType]
pub enum FsEntry {
    File(Hosts),
    Directory(Vec<Ino>),
    Symlink(EntrySymlink),
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
