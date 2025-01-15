use crate::pods::arbo::{FsEntry, Inode};
use crate::pods::fs_interface::{FsInterface, SimpleFileType};
use crate::pods::whpath::WhPath;
use fuser::{
    BackgroundSession, FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEntry, Request,
};
use libc::{ENOENT, ENOSYS};
use log::debug;
use openat::{Dir, SimpleType};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

// NOTE - placeholders
const TTL: Duration = Duration::from_secs(1);

const MOUNT_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

pub const TEMPLATE_FILE_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};
// ^ placeholders

// const MIRROR_PTH: &str = "./wh_mirror/";

pub struct FuseController {
    pub fs_interface: Arc<FsInterface>,
}

// NOTE for dev purpose while all metadata is not supported
fn inode_to_fuse_fileattr(inode: Inode) -> FileAttr {
    let mut attr = TEMPLATE_FILE_ATTR;
    attr.ino = inode.id;
    attr.kind = match inode.entry {
        FsEntry::Directory(_) => fuser::FileType::Directory,
        FsEntry::File(_) => fuser::FileType::RegularFile,
    };
    attr
}

// REVIEW - should later invest in proper error handling
impl Filesystem for FuseController {
    // READING

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        match self
            .fs_interface
            .get_entry_from_name(parent, name.to_string_lossy().to_string())
        {
            Ok(inode) => {
                reply.entry(&TTL, &inode_to_fuse_fileattr(inode), 0);
            }
            Err(_) => {
                reply.error(ENOENT);
            }
        };
    }

    // TODO
    fn getattr(&mut self, _req: &Request, ino: u64, _: Option<u64>, reply: ReplyAttr) {
        debug!("getattr is called {}", ino);
        match ino {
            1 => reply.attr(&TTL, &MOUNT_DIR_ATTR),
            2 => reply.attr(&TTL, &TEMPLATE_FILE_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        let content = self.fs_interface.read_file(
            ino,
            offset.try_into().expect("fuse_impl::read offset negative"),
            size.try_into().expect("fuse_impl::read size too large"),
        );

        match content {
            Ok(content) => reply.data(&content),
            Err(_) => reply.error(ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let entries = if let Ok(entries) = self.fs_interface.read_dir(ino) {
            entries
        } else {
            reply.error(ENOENT);
            return;
        };

        let mut i = offset;
        for entry in entries.into_iter().skip(offset as usize) {
            println!("....readdir entries : {:?}", entry);
            if reply.add(
                ino,
                (i) as i64, // NOTE - in case of error, try i + 1
                entry.entry.get_filetype(),
                entry.name,
            ) {
                break;
            }
            i += 1;
        }
        reply.ok();
    }

    // ^ READING

    // WRITING

    fn mknod(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        if let Ok((id, _)) = self.fs_interface.make_inode(
            parent,
            name.to_string_lossy().to_string(),
            SimpleFileType::File,
        ) {
            // creating metadata to return
            let mut new_attr = TEMPLATE_FILE_ATTR;
            new_attr.ino = id;
            new_attr.kind = FileType::RegularFile;
            new_attr.size = 0;

            reply.entry(&TTL, &new_attr, 0)
        } else {
            reply.error(ENOSYS)
        }
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        if let Ok((id, _)) = self.fs_interface.make_inode(
            parent,
            name.to_string_lossy().to_string(),
            SimpleFileType::File,
        ) {
            // creating metadata to return
            let mut new_attr = TEMPLATE_FILE_ATTR;
            new_attr.ino = id;
            new_attr.kind = FileType::Directory;
            new_attr.size = 0;

            reply.entry(&TTL, &new_attr, 0)
        } else {
            reply.error(ENOSYS)
        }
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        if let Ok(()) = self.fs_interface.fuse_remove_inode(parent, name) {
            reply.ok()
        } else {
            reply.error(ENOENT)
        }
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        // should be only called on empty dirs ?
        if let Ok(()) = self.fs_interface.fuse_remove_inode(parent, name) {
            reply.ok()
        } else {
            reply.error(ENOENT)
        }
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        reply.error(ENOENT) // TODO
                            // let mut provider = self.provider.lock().unwrap();
                            // if let Some(()) = provider.rename(parent, name, newparent, newname) {
                            //     reply.ok()
                            // } else {
                            //     reply.error(ENOENT)
                            // }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        let offset = offset
            .try_into()
            .expect("fuser write: can't convert i64 to u64");

        if let Ok(written) = self.fs_interface.write(ino, data.to_vec(), offset) {
            reply.written(
                written
                    .try_into()
                    .expect("fuser write: can't convert u64 to u32"),
            )
        } else {
            reply.error(ENOENT)
        }
    }

    // ^ WRITING
}

pub fn mount_fuse(
    mount_point: &WhPath,
    fs_interface: Arc<FsInterface>,
) -> io::Result<BackgroundSession> {
    let options = vec![MountOption::RW, MountOption::FSName("wormhole".to_string())];
    let ctrl = FuseController { fs_interface };

    fuser::spawn_mount2(ctrl, mount_point.to_string(), &options)
}
