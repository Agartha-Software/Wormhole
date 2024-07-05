use fuser::{
    BackgroundSession, FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEntry, Request,
};
use futures_util::future::Fuse;
use libc::{ENOENT, ENOSYS};
use log::debug;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::mpsc::UnboundedSender;

use crate::network::message::NetworkMessage;
use crate::network::server::State;
use crate::providers::{FsIndex, Provider};

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

const TEMPLATE_FILE_ATTR: FileAttr = FileAttr {
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

// const COPIED_ROOT: &str = "./original/";

pub struct FuseController {
    pub provider: Arc<Mutex<Provider>>,
}

// Custom data (not directly linked to fuse)
// create some data for us like the index or data provider
impl FuseController {
    // for the mirror version
    // we create an index of the original folder
    fn index_folder() -> FsIndex {
        let mut arbo: FsIndex = HashMap::new();
        let mut inode: u64 = 2;

        // arbo.insert(1, (fuser::FileType::Directory, COPIED_ROOT.to_owned()));

        // for entry in WalkDir::new(COPIED_ROOT).into_iter().filter_map(|e| e.ok()) {
        //     let strpath = entry.path().display().to_string();
        //     let path_type = if entry.file_type().is_dir() {
        //         fuser::FileType::Directory
        //     } else if entry.file_type().is_file() {
        //         fuser::FileType::RegularFile
        //     } else {
        //         fuser::FileType::CharDevice // random to detect unsupported
        //     };
        //     if strpath != COPIED_ROOT && path_type != fuser::FileType::CharDevice {
        //         debug!("indexing {}", strpath);
        //         arbo.insert(inode, (path_type, strpath));
        //         inode += 1;
        //     } else {
        //         debug!("ignoring {}", strpath);
        //     }
        // }
        arbo
    }
}

impl Filesystem for FuseController {
    // READING

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        debug!("lookup is called {} {:?}", parent, name);
        let provider = self.provider.lock().unwrap();
        if let Some(file_attr) = provider.fs_lookup(parent, name) {
            reply.entry(&TTL, &file_attr, 0)
        } else {
            reply.error(ENOENT)
        }
    }

    // TODO
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
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
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        debug!("read is called");
        let provider = self.provider.lock().unwrap();
        if let Some(content) = provider.read(ino) {
            reply.data(&content[offset as usize..])
        } else {
            reply.error(ENOENT);
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
        debug!("readdir is called for ino {}", ino);
        let provider = self.provider.lock().unwrap();
        if let Some(entries) = provider.fs_readdir(ino) {
            for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
                debug!("readdir entries : {:?}", entry);
                // i + 1 means the index of the next entry
                if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }
            reply.ok()
        } else {
            debug!("readdir EONENT ");
            reply.error(ENOENT)
        }
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
        let mut provider = self.provider.lock().unwrap();
        if let Some(attr) = provider.mkfile(parent, name) {
            reply.entry(&TTL, &attr, 0)
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
        let mut provider = self.provider.lock().unwrap();
        if let Some(attr) = provider.mkdir(parent, name) {
            reply.entry(&TTL, &attr, 0)
        } else {
            reply.error(ENOSYS)
        }
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        let mut provider = self.provider.lock().unwrap();
        if let Some(()) = provider.rmfile(parent, name) {
            reply.ok()
        } else {
            reply.error(ENOENT)
        }
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        // should be only called on empty dirs ?
        let mut provider = self.provider.lock().unwrap();
        if let Some(()) = provider.rmdir(parent, name) {
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
        // comment sont gérés les dossiers et sous fichiers ?
        let mut provider = self.provider.lock().unwrap();
        if let Some(()) = provider.rename(parent, name, newparent, newname) {
            reply.ok()
        } else {
            reply.error(ENOENT)
        }
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
        let provider = self.provider.lock().unwrap();
        if let Some(written) = provider.write(ino, offset, data) {
            reply.written(written)
        } else {
            reply.error(ENOENT)
        }
    }

    // ^ WRITING
}

pub fn mount_fuse(
    state: &Arc<State>,
    mountpoint: &str,
    tx: UnboundedSender<NetworkMessage>,
) -> (BackgroundSession, Arc<Mutex<Provider>>) {
    let options = vec![MountOption::RW, MountOption::FSName("wormhole".to_string())];
    let index = FuseController::index_folder();
    let provider = Arc::new(Mutex::new(Provider {
        next_inode: (index.len() + 2) as u64,
        index,
        tx,
        state: state.clone()
    }));
    let ctrl = FuseController {
        provider: provider.clone(),
    };
    (
        fuser::spawn_mount2(ctrl, mountpoint, &options).unwrap(),
        provider,
    )
}
