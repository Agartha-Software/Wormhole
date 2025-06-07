use crate::fuse::linux_attrs::time_or_now_to_system_time;
use crate::fuse::linux_mknod::filetype_from_mode;
use crate::pods::arbo::{FsEntry, Inode};
use crate::pods::filesystem::attrs::SetAttrError;
use crate::pods::filesystem::fs_interface::{FsInterface, SimpleFileType};
use crate::pods::filesystem::make_inode::{CreateError, MakeInodeError};
use crate::pods::filesystem::open::OpenError;
use crate::pods::filesystem::read::ReadError;
use crate::pods::filesystem::remove_inode::RemoveFileError;
use crate::pods::filesystem::rename::RenameError;
use crate::pods::filesystem::write::WriteError;
use crate::pods::filesystem::xattrs::GetXAttrError;
use crate::pods::network::pull_file::PullError;
use crate::pods::whpath::WhPath;
use fuser::{
    BackgroundSession, FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyXattr, Request,
};
use libc::{EIO, ENOENT, XATTR_CREATE, XATTR_REPLACE};
use std::ffi::OsStr;
use std::io;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// NOTE - placeholders
const TTL: Duration = Duration::from_secs(1);

impl Into<FileType> for SimpleFileType {
    fn into(self) -> FileType {
        match self {
            SimpleFileType::File => FileType::RegularFile,
            SimpleFileType::Directory => FileType::Directory,
        }
    }
}

impl Into<SimpleFileType> for FileType {
    fn into(self) -> SimpleFileType {
        match self {
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

pub struct FuseController {
    pub fs_interface: Arc<FsInterface>,
}

// NOTE for dev purpose while all metadata is not supported
fn inode_to_fuse_fileattr(inode: Inode) -> FileAttr {
    let mut attr: FileAttr = inode.meta.into();
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

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let attrs = self.fs_interface.get_inode_attributes(ino);

        match attrs {
            Ok(attrs) => reply.attr(&TTL, &attrs.into()),
            Err(err) => {
                log::error!("getattr error: {:?}", err);
                reply.error(err.raw_os_error().unwrap_or(EIO))
            }
        }
    }

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<fuser::TimeOrNow>,
        mtime: Option<fuser::TimeOrNow>,
        ctime: Option<SystemTime>,
        file_handle: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        match self.fs_interface.setattr(
            ino,
            mode,
            uid,
            gid,
            size,
            atime.map(|time| time_or_now_to_system_time(time)),
            mtime.map(|time| time_or_now_to_system_time(time)),
            ctime,
            file_handle,
            flags,
        ) {
            Ok(meta) => reply.attr(&TTL, &meta.into()),
            Err(SetAttrError::WhError { source }) => reply.error(source.to_libc()),
            Err(SetAttrError::SizeNoPerm) => reply.error(libc::EPERM),
            Err(SetAttrError::InvalidFileHandle) => reply.error(libc::EBADFD),
            Err(SetAttrError::SetFileSizeIoError { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local setattr error should always be the underling libc::open os error",
                ))
            }
            Err(SetAttrError::SetPermIoError { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local setattr error should always be the underling libc::open os error",
                ))
            }
        }
    }

    fn getxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        size: u32,
        reply: ReplyXattr,
    ) {
        let attr = self
            .fs_interface
            .get_inode_xattr(ino, &name.to_string_lossy().to_string());

        let data = match attr {
            Ok(data) => data,
            Err(GetXAttrError::KeyNotFound) => {
                reply.error(libc::ERANGE);
                return;
            }
            Err(GetXAttrError::WhError { source }) => {
                reply.error(source.to_libc());
                return;
            }
        };

        if size == 0 {
            reply.size(data.len() as u32);
        } else {
            reply.data(&data);
        }
    }

    fn setxattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        name: &OsStr,
        data: &[u8],
        flags: i32,
        _position: u32, // Postion undocumented
        reply: ReplyEmpty,
    ) {
        // As we follow linux implementation in spirit, data size limit at 64kb
        if data.len() > 64000 {
            return reply.error(libc::ENOSPC);
        }

        let key = name.to_string_lossy().to_string();

        if flags == XATTR_CREATE || flags == XATTR_REPLACE {
            match self.fs_interface.xattr_exists(ino, &key) {
                Ok(true) => {
                    if flags == XATTR_CREATE {
                        return reply.error(libc::EEXIST);
                    }
                }
                Ok(false) => {
                    if flags == XATTR_REPLACE {
                        return reply.error(libc::ENODATA);
                    }
                }
                Err(err) => {
                    return reply.error(err.to_libc());
                }
            }
        }

        // TODO - Implement After permission implementation
        // let attr = self.fs_interface.get_inode_attributes(ino);
        // if attr.unwrap().perm == valid {
        //     reply.error(libc::EPERM);
        // }

        match self
            .fs_interface
            .network_interface
            .set_inode_xattr(ino, key, data.to_vec())
        {
            Ok(_) => reply.ok(),
            Err(err) => reply.error(err.to_libc()),
        }
    }

    fn removexattr(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        match self
            .fs_interface
            .network_interface
            .remove_inode_xattr(ino, name.to_string_lossy().to_string())
        {
            Ok(_) => reply.ok(),
            Err(err) => reply.error(err.to_libc()),
        }
    }

    fn listxattr(&mut self, _req: &Request<'_>, ino: u64, size: u32, reply: ReplyXattr) {
        match self.fs_interface.list_inode_xattr(ino) {
            Ok(keys) => {
                let mut bytes = vec![];

                for key in keys {
                    bytes.extend(key.bytes());
                    bytes.push(0);
                }
                if size == 0 {
                    reply.size(bytes.len() as u32);
                } else if size >= bytes.len() as u32 {
                    reply.data(&bytes);
                } else {
                    reply.error(libc::ERANGE)
                }
                return;
            }
            Err(err) => match err.to_libc() {
                libc::ENOENT => reply.error(libc::EBADF), // Not found became Bad file descriptor
                or => reply.error(or),
            },
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        file_handle: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        let mut buf = vec![];
        buf.resize(size as usize, 0);
        match self.fs_interface.read_file(
            ino,
            offset.try_into().expect("read::read offset negative"),
            &mut buf,
            file_handle,
        ) {
            Ok(size) => {
                buf.resize(size, 0);
                reply.data(&buf)
            }
            Err(ReadError::WhError { source }) => reply.error(source.to_libc()),
            Err(ReadError::PullError {
                source: PullError::WhError { source },
            }) => reply.error(source.to_libc()),
            Err(ReadError::CantPull) => reply.error(libc::ENETUNREACH),
            Err(ReadError::LocalReadFailed { io }) => reply.error(
                io.raw_os_error()
                    .expect("Local read error should always be the underling libc::open os error"),
            ),
            Err(ReadError::PullError {
                source: PullError::NoHostAvailable,
            }) => reply.error(libc::ENETUNREACH),
            Err(ReadError::NoFileHandle) => reply.error(libc::EBADFD), // Shouldn't happend
            //According to the man EBADF if the fd is not a valid file descriptor or is not open for reading.
            Err(ReadError::NoReadPermission) => reply.error(libc::EBADFD),
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
        let entries = match self.fs_interface.read_dir(ino) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!("readdir: ENOENT {e} {ino}");
                reply.error(ENOENT);
                return;
            }
        };

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(
                ino,
                // i + 1 means offset of the next entry
                i as i64 + 1, // NOTE - in case of error, try i + 1
                entry.entry.get_filetype().into(),
                entry.name,
            ) {
                break;
            }
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
        mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        let permissions = mode as u16;
        let kind = match filetype_from_mode(mode) {
            Some(kind) => kind,
            None => {
                // If it's not a file or a directory it's not yet supported
                reply.error(libc::ENOSYS);
                return;
            }
        };

        match self.fs_interface.make_inode(
            parent,
            name.to_string_lossy().to_string(),
            permissions,
            kind,
        ) {
            Ok(node) => reply.entry(&TTL, &node.meta.into(), 0),
            Err(MakeInodeError::LocalCreationFailed { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local creation error should always be the underling libc::open os error",
                ))
            }
            Err(MakeInodeError::WhError { source }) => reply.error(source.to_libc()),
            Err(MakeInodeError::AlreadyExist) => reply.error(libc::EEXIST),
            Err(MakeInodeError::ParentNotFound) => reply.error(libc::ENOENT),
            Err(MakeInodeError::ParentNotFolder) => reply.error(libc::ENOTDIR),
            Err(MakeInodeError::ProtectedNameIsFolder) => reply.error(libc::EISDIR),
        }
        //todo when persmissions are added reply.error(libc::EACCES)
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        match self.fs_interface.make_inode(
            parent,
            name.to_string_lossy().to_string(),
            mode as u16,
            SimpleFileType::Directory,
        ) {
            Ok(node) => reply.entry(&TTL, &node.meta.into(), 0),
            Err(MakeInodeError::LocalCreationFailed { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local creation error should always be the underling libc::open os error",
                ))
            }
            Err(MakeInodeError::WhError { source }) => reply.error(source.to_libc()),
            Err(MakeInodeError::AlreadyExist) => reply.error(libc::EEXIST),
            Err(MakeInodeError::ParentNotFound) => reply.error(libc::ENOENT),
            Err(MakeInodeError::ParentNotFolder) => reply.error(libc::ENOTDIR),
            Err(MakeInodeError::ProtectedNameIsFolder) => reply.error(libc::EISDIR),
        }
    }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        match self.fs_interface.fuse_remove_inode(parent, name) {
            Ok(()) => reply.ok(),
            Err(RemoveFileError::WhError { source }) => reply.error(source.to_libc()),
            Err(RemoveFileError::LocalDeletionFailed { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local creation error should always be the underling libc::open os error",
                ))
            }
            Err(RemoveFileError::NonEmpty) => reply.error(libc::ENOTEMPTY),
        }
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        match self.fs_interface.fuse_remove_inode(parent, name) {
            Ok(()) => reply.ok(),
            Err(RemoveFileError::WhError { source }) => reply.error(source.to_libc()),
            Err(RemoveFileError::LocalDeletionFailed { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local creation error should always be the underling libc::open os error",
                ))
            }
            Err(RemoveFileError::NonEmpty) => reply.error(libc::ENOTEMPTY),
        }
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        newname: &OsStr,
        flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        match self
            .fs_interface
            .rename(
                parent,
                new_parent,
                &name //TODO move instead of ref because of the clone down the line
                    .to_owned()
                    .into_string()
                    .expect("Don't support non unicode yet"), //TODO support OsString smartly
                &newname
                    .to_owned()
                    .into_string()
                    .expect("Don't support non unicode yet"),
                flags & libc::RENAME_NOREPLACE == 0,
            )
            .inspect_err(|err| log::error!("rename: {err}"))
        {
            Ok(()) => reply.ok(),
            Err(RenameError::WhError { source }) => reply.error(source.to_libc()),
            Err(RenameError::LocalRenamingFailed { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local renaming error should always be the underling libc::open os error",
                ))
            }
            Err(RenameError::LocalOverwriteFailed { io }) => reply.error(io.raw_os_error().expect(
                "Local overwrite error should always be the underling libc::open os error",
            )),
            Err(RenameError::OverwriteNonEmpty) => reply.error(libc::ENOTEMPTY),
            Err(RenameError::DestinationExists) => reply.error(libc::EEXIST),
            Err(RenameError::SourceParentNotFolder) => reply.error(libc::ENOTDIR),
            Err(RenameError::SourceParentNotFound) => reply.error(libc::ENOENT),
            Err(RenameError::DestinationParentNotFolder) => reply.error(libc::ENOTDIR),
            Err(RenameError::DestinationParentNotFound) => reply.error(libc::ENOENT),
            Err(RenameError::ProtectedNameIsFolder) => reply.error(libc::ENOTDIR),
            Err(RenameError::ReadFailed { source: _ }) => reply.error(libc::EIO), // TODO
            Err(RenameError::LocalWriteFailed { io }) => reply.error(
                io.raw_os_error()
                    .expect("Local read error should always be the underling os error"),
            ),
        }
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: fuser::ReplyOpen) {
        match self.fs_interface.open(ino, flags) {
            Ok(file_handle) => reply.opened(file_handle, flags as u32), // TODO - check flags ?,
            Err(OpenError::WhError { source }) => reply.error(source.to_libc()),
            Err(OpenError::MultipleAccessFlags) => reply.error(libc::EINVAL),
            Err(OpenError::TruncReadOnly) => reply.error(libc::EACCES),
            Err(OpenError::WrongPermissions) => reply.error(libc::EPERM),
        };
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        file_handle: u64,
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

        match self.fs_interface.write(ino, data, offset, file_handle) {
            Ok(written) => reply.written(
                written
                    .try_into()
                    .expect("fuser write: can't convert u64 to u32"),
            ),
            Err(WriteError::WhError { source }) => reply.error(source.to_libc()),
            Err(WriteError::LocalWriteFailed { io }) => {
                reply.error(io.raw_os_error().expect(
                    "Local creation error should always be the underling libc::open os error",
                ))
            }
            Err(WriteError::NoFileHandle) => reply.error(libc::EBADFD), // Shouldn't happend
            //According to the man EBADF if the fd is not a valid file descriptor or is not open for writing.
            Err(WriteError::NoWritePermission) => reply.error(libc::EBADF), // Shouldn't happend, write not call with wrong perms, already stopped
        }
    }

    // ^ WRITING

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        let permissions = mode as u16;
        let kind = match filetype_from_mode(mode) {
            Some(kind) => kind,
            None => {
                // If it's not a file or a directory it's not yet supported
                reply.error(libc::ENOSYS);
                return;
            }
        };

        match self.fs_interface.create(
            parent,
            name.to_string_lossy().to_string(),
            flags,
            permissions,
            kind,
        ) {
            Ok((inode, fh)) => reply.created(&TTL, &inode.meta.into(), 0, fh, flags as u32),
            Err(CreateError::MakeInode {
                source: MakeInodeError::LocalCreationFailed { io },
            }) => {
                reply.error(io.raw_os_error().expect(
                    "Local creation error should always be the underling libc::open os error",
                ))
            }
            Err(CreateError::MakeInode {
                source: MakeInodeError::WhError { source },
            }) => reply.error(source.to_libc()),
            Err(CreateError::MakeInode {
                source: MakeInodeError::AlreadyExist,
            }) => reply.error(libc::EEXIST),
            Err(CreateError::MakeInode {
                source: MakeInodeError::ParentNotFound,
            }) => reply.error(libc::ENOENT),
            Err(CreateError::MakeInode {
                source: MakeInodeError::ParentNotFolder,
            }) => reply.error(libc::ENOTDIR),
            Err(CreateError::MakeInode {
                source: MakeInodeError::ProtectedNameIsFolder,
            }) => reply.error(libc::EISDIR),
            Err(CreateError::WhError { source }) => reply.error(source.to_libc()),
            Err(CreateError::OpenError {
                source: OpenError::WhError { source },
            }) => reply.error(source.to_libc()),
            Err(CreateError::OpenError {
                source: OpenError::MultipleAccessFlags,
            }) => reply.error(libc::EINVAL),
            Err(CreateError::OpenError {
                source: OpenError::TruncReadOnly,
            }) => reply.error(libc::EACCES),
            Err(CreateError::OpenError {
                source: OpenError::WrongPermissions,
            }) => reply.error(libc::EPERM),
        }
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        file_handle: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        match self.fs_interface.release(file_handle) {
            Ok(()) => reply.ok(),
            Err(err) => reply.error(err.to_libc()),
        }
    }
}

pub fn mount_fuse(
    mount_point: &WhPath,
    fs_interface: Arc<FsInterface>,
) -> io::Result<BackgroundSession> {
    let options = vec![
        MountOption::RW,
        // MountOption::DefaultPermissions,
        MountOption::FSName("wormhole".to_string()),
    ];
    let ctrl = FuseController { fs_interface };

    fuser::spawn_mount2(ctrl, mount_point.to_string(), &options)
}
