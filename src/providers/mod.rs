use serde::{Deserialize, Serialize};
// use openat::Dir;
use std::{collections::HashMap, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use tokio::sync::mpsc::UnboundedSender;

use crate::network::message::NetworkMessage;
use handle::FolderHandle;

mod helpers;
mod handle;
pub mod readers;
pub mod writers;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FileType {
    RegularFile,
    Directory,
    Link,
    Other,
}

/// File attributes
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileAttr {
    /// Inode number
    pub ino: u64,
    /// Size in bytes
    pub size: u64,
    /// Size in blocks
    pub blocks: u64,
    /// Time of last access
    pub atime: SystemTime,
    /// Time of last modification
    pub mtime: SystemTime,
    /// Time of last change
    pub ctime: SystemTime,
    /// Time of creation (macOS only)
    pub crtime: SystemTime,
    /// Kind of file (directory, file, pipe, etc)
    pub kind: FileType,
    /// Permissions
    pub perm: u16,
    /// Number of hard links
    pub nlink: u32,
    /// User id
    pub uid: u32,
    /// Group id
    pub gid: u32,
    /// Rdev
    pub rdev: u32,
    /// Block size
    pub blksize: u32,
    /// Flags (macOS only, see chflags(2))
    pub flags: u32,
}

// (inode_number, (Type, Original path))
pub type FsIndex = HashMap<u64, (FileType, PathBuf)>;

// will keep all the necessary info to provide real
// data to the fuse lib
// For now this is given to the fuse controler on creation and we do NOT have
// ownership during the runtime.
pub struct Provider {
    pub next_inode: u64,
    pub index: FsIndex,
    pub local_source: PathBuf,
    pub folder_handle: Box<dyn FolderHandle + Send>,
    pub tx: UnboundedSender<NetworkMessage>,
}

// will soon be replaced once the dev continues
const TEMPLATE_FILE_ATTR: FileAttr = FileAttr {
    ino: 2,   // required to be correct
    size: 13, // required to be correct
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile, // required to be correct
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

impl Provider {}
