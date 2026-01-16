use std::{ops::Deref, time::SystemTime};

use fuser::{FileAttr, FileType, TimeOrNow};

use crate::pods::{filesystem::fs_interface::SimpleFileType, itree::Metadata};

impl From<SimpleFileType> for FileType {
    fn from(val: SimpleFileType) -> FileType {
        match val {
            SimpleFileType::File => FileType::RegularFile,
            SimpleFileType::Directory => FileType::Directory,
        }
    }
}

impl From<&SimpleFileType> for FileType {
    fn from(val: &SimpleFileType) -> Self {
        match val {
            SimpleFileType::File => FileType::RegularFile,
            SimpleFileType::Directory => FileType::Directory,
        }
    }
}

impl From<FileType> for SimpleFileType {
    fn from(val: FileType) -> Self {
        match val {
            FileType::RegularFile => SimpleFileType::File,
            FileType::Directory => SimpleFileType::Directory,
            FileType::NamedPipe => todo!("file type not supported"),
            FileType::CharDevice => todo!("file type not supported"),
            FileType::BlockDevice => todo!("file type not supported"),
            FileType::Symlink => todo!("file type not supported"),
            FileType::Socket => todo!("file type not supported"),
        }
    }
}

struct MetadataFileAttr<'a>(&'a Metadata);

impl<'a> Deref for MetadataFileAttr<'a> {
    type Target = Metadata;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> From<&MetadataFileAttr<'a>> for FileAttr {
    fn from(val: &MetadataFileAttr<'a>) -> Self {
        FileAttr {
            ino: val.ino,
            size: val.size,
            blocks: val.size,
            atime: val.atime,
            mtime: val.mtime,
            ctime: val.ctime,
            crtime: val.crtime,
            kind: (&val.kind).into(),
            perm: val.perm,
            nlink: val.nlink,
            uid: val.uid,
            gid: val.gid,
            rdev: val.rdev,
            flags: val.flags,
            blksize: val.blksize,
        }
    }
}

impl From<FileAttr> for Metadata {
    fn from(val: FileAttr) -> Self {
        Metadata {
            ino: val.ino,
            size: val.size,
            blocks: val.blocks,
            atime: val.atime,
            mtime: val.mtime,
            ctime: val.ctime,
            crtime: val.crtime,
            kind: val.kind.into(),
            perm: val.perm,
            nlink: val.nlink,
            uid: val.uid,
            gid: val.gid,
            rdev: val.rdev,
            flags: val.flags,
            blksize: val.blksize,
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
