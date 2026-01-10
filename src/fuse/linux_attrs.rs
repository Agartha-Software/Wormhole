use std::{ops::Deref, time::SystemTime};

use fuser::{FileAttr, FileType, TimeOrNow};

use crate::pods::{filesystem::fs_interface::SimpleFileType, itree::Metadata};

impl Into<FileType> for SimpleFileType {
    fn into(self) -> FileType {
        match self {
            SimpleFileType::File => FileType::RegularFile,
            SimpleFileType::Directory => FileType::Directory,
            SimpleFileType::Symlink => FileType::Symlink,
        }
    }
}

impl Into<FileType> for &SimpleFileType {
    fn into(self) -> FileType {
        match self {
            SimpleFileType::File => FileType::RegularFile,
            SimpleFileType::Directory => FileType::Directory,
            SimpleFileType::Symlink => FileType::Symlink,
        }
    }
}

impl Into<SimpleFileType> for FileType {
    fn into(self) -> SimpleFileType {
        match self {
            FileType::RegularFile => SimpleFileType::File,
            FileType::Directory => SimpleFileType::Directory,
            FileType::Symlink => SimpleFileType::Symlink,
            FileType::NamedPipe => todo!("file type not supported"),
            FileType::CharDevice => todo!("file type not supported"),
            FileType::BlockDevice => todo!("file type not supported"),
            FileType::Socket => todo!("file type not supported"),
        }
    }
}

struct MetadataFileAttr<'a>(&'a Metadata);

impl<'a> Deref for MetadataFileAttr<'a> {
    type Target = Metadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Into<FileAttr> for &MetadataFileAttr<'a> {
    fn into(self) -> FileAttr {
        FileAttr {
            ino: self.ino,
            size: self.size,
            blocks: self.size,
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            crtime: self.crtime,
            kind: (&self.kind).into(),
            perm: self.perm,
            nlink: self.nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: self.rdev,
            flags: self.flags,
            blksize: self.blksize,
        }
    }
}

impl Into<Metadata> for FileAttr {
    fn into(self) -> Metadata {
        Metadata {
            ino: self.ino,
            size: self.size,
            blocks: self.blocks,
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            crtime: self.crtime,
            kind: self.kind.into(),
            perm: self.perm,
            nlink: self.nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: self.rdev,
            flags: self.flags,
            blksize: self.blksize,
        }
    }
}

impl Metadata {
    pub fn with_ids(&self, uid: u32, gid: u32) -> FileAttr {
        let mut attr: FileAttr = (&MetadataFileAttr(self)).into();
        attr.uid = uid;
        attr.gid = gid;
        attr
    }
}

pub fn time_or_now_to_system_time(time: TimeOrNow) -> SystemTime {
    match time {
        TimeOrNow::Now => SystemTime::now(),
        TimeOrNow::SpecificTime(time) => time,
    }
}
