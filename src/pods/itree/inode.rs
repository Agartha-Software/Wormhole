use std::fs;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use std::{collections::HashMap, time::SystemTime};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::pods::{
    filesystem::fs_interface::SimpleFileType,
    itree::{FsEntry, BLOCK_SIZE},
    whpath::InodeName,
};

/// Ino is short for an Inode's id number. I can be thought as i-number (ino)
/// This is the conventional name for inode's number across unix systems
pub type Ino = u64;

// root ino is always 1. Other ino are dynamically assigned
pub const ROOT: Ino = 1;

pub type XAttrs = HashMap<String, Vec<u8>>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Inode {
    pub parent: Ino,
    pub id: Ino,
    pub name: InodeName,
    pub entry: FsEntry,
    pub meta: Metadata,
    pub xattrs: XAttrs,
}

impl Inode {
    pub fn new(name: InodeName, parent_ino: Ino, id: Ino, entry: FsEntry, perm: u16) -> Self {
        let meta = Metadata {
            ino: id,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: entry.get_filetype(),
            perm,
            nlink: 1 + matches!(entry, FsEntry::Directory(_)) as u32,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: BLOCK_SIZE as u32,
            flags: 0,
        };

        let xattrs = HashMap::new();

        Self {
            parent: parent_ino,
            id,
            name,
            entry,
            meta,
            xattrs,
        }
    }
}

pub const WINDOWS_DEFAULT_PERMS_MODE: u16 = 0o666;

/* NOTE
 * is currently made with fuse in sight. Will probably need to be edited to be windows compatible
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, TS)]
pub struct Metadata {
    /// Inode number
    pub ino: u64,
    /// Size in bytes
    pub size: u64,
    /// Size in blocks
    pub blocks: u64,
    /// Time of last access
    #[ts(as = "u64")]
    pub atime: SystemTime,
    /// Time of last modification
    #[ts(as = "u64")]
    pub mtime: SystemTime,
    /// Time of last change
    #[ts(as = "u64")]
    pub ctime: SystemTime,
    /// Time of creation (macOS only)
    #[ts(as = "u64")]
    pub crtime: SystemTime,
    /// Kind of file (directory, file, pipe, etc)
    pub kind: SimpleFileType,
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

#[cfg(target_os = "linux")]
impl TryInto<Metadata> for fs::Metadata {
    type Error = std::io::Error;
    fn try_into(self) -> Result<Metadata, std::io::Error> {
        Ok(Metadata {
            ino: 0, // TODO: unsafe default
            size: self.len(),
            blocks: 0,
            atime: self.accessed()?,
            mtime: self.modified()?,
            ctime: self.modified()?,
            crtime: self.created()?,
            kind: if self.is_file() {
                SimpleFileType::File
            } else {
                SimpleFileType::Directory
            },
            perm: self.permissions().mode() as u16,
            nlink: self.nlink() as u32,
            uid: self.uid(),
            gid: self.gid(),
            rdev: self.rdev() as u32,
            blksize: self.blksize() as u32,
            flags: 0,
        })
    }
}

#[cfg(target_os = "windows")]
impl TryInto<Metadata> for fs::Metadata {
    type Error = std::io::Error;
    fn try_into(self) -> Result<Metadata, std::io::Error> {
        Ok(Metadata {
            ino: 0, // TODO: unsafe default
            size: self.len(),
            blocks: 0,
            atime: self.accessed()?,
            mtime: self.modified()?,
            ctime: self.modified()?,
            crtime: self.created()?,
            kind: if self.is_file() {
                SimpleFileType::File
            } else {
                SimpleFileType::Directory
            },
            perm: WINDOWS_DEFAULT_PERMS_MODE,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            flags: 0,
        })
    }
}
