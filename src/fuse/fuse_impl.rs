use fuser::{
    BackgroundSession, FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};
use walkdir::WalkDir;

use crate::data::readers::{FsIndex, Provider};

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

const TEMPLATE_FILE_CONTENT: &str = "Hello World!\n";

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

const COPIED_ROOT: &str = "./original/";
pub struct FuseController {
    pub outside_info: Provider,
}

impl FuseController {
    fn new() -> Self {
        Self {
            outside_info: Provider {
                index: Self::index_folder(),
            },
        }
    }

    fn index_folder() -> FsIndex {
        let mut arbo: FsIndex = HashMap::new();
        let mut inode: u64 = 2;

        arbo.insert(1, (fuser::FileType::Directory, COPIED_ROOT.to_owned()));

        for entry in WalkDir::new(COPIED_ROOT).into_iter().filter_map(|e| e.ok()) {
            let strpath = entry.path().display().to_string();
            let path_type = if entry.file_type().is_dir() {
                fuser::FileType::Directory
            } else if entry.file_type().is_file() {
                fuser::FileType::RegularFile
            } else {
                fuser::FileType::CharDevice // random to detect unsupported
            };
            if strpath != COPIED_ROOT && path_type != fuser::FileType::CharDevice {
                println!("indexing {}", strpath);
                arbo.insert(inode, (path_type, strpath));
                inode += 1;
            } else {
                println!("ignoring {}", strpath);
            }
        }
        arbo
    }
}

impl Filesystem for FuseController {
    // Look up a directory entry by name and get its attributes.
    // parent = folder inode ? | name = file/folder (not path)
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        println!("lookup is called {} {:?}", parent, name);
        if parent == 1 && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &TEMPLATE_FILE_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        println!("getattr is called {}", ino);
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
        println!("read is called");
        if ino == 2 {
            reply.data(&TEMPLATE_FILE_CONTENT.as_bytes()[offset as usize..]);
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
        // if ino != 1 {
        //     reply.error(ENOENT);
        //     return;
        // }

        // let entries = vec![
        //     (1, FileType::Directory, "."),
        //     (1, FileType::Directory, ".."),
        //     (2, FileType::RegularFile, "hello.txt"),
        // ];

        println!("readdir is called for ino {}", ino);
        if let Some(entries) = self.outside_info.fs_readdir(ino) {
            for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
                println!("readdir entries : {:?}", entry);
                // i + 1 means the index of the next entry
                if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }
            reply.ok()
        } else {
            println!("readdir EONENT ");
            reply.error(ENOENT)
        }
    }
}

pub fn mount_fuse(mountpoint: &String) -> BackgroundSession {
    let options = vec![MountOption::RO, MountOption::FSName("wormhole".to_string())];
    // options.push(MountOption::AllowOther);/
    let ctrl = FuseController::new();
    fuser::spawn_mount2(ctrl, mountpoint, &options).unwrap() // FIXME unwrap
}