use std::time::SystemTime;

use custom_error::custom_error;

use crate::{
    config::{types::Config, LocalConfig},
    error::WhError,
    pods::{
        arbo::{Arbo, FsEntry, InodeId, Metadata, BLOCK_SIZE},
        filesystem::{
            file_handle::{AccessMode, FileHandleManager, UUID},
            fs_interface::FsInterface,
            permissions::has_write_perm,
        },
    },
};

custom_error! {pub SetAttrError
    WhError{source: WhError} = "{source}",
    SizeNoPerm = "Edit size require to have the write permission on the file",
    InvalidFileHandle = "File handle not found in the open file handles",
    SetFileSizeIoError { io: std::io::Error } = "Set file size disk side failed: {io}",
    SetPermIoError { io: std::io::Error } = "Set file size disk side failed : {io}"
}

custom_error! {pub AcknoledgeSetAttrError
    WhError{source: WhError} = "{source}",
    SetFileSizeIoError {io: std::io::Error } = "Set file size disk side failed: {io}"
}

impl FsInterface {
    //fn get_inode_attributes(&self, ino: InodeId) -> WhResult<&Metadata> {}

    pub fn acknowledge_metadata(
        &self,
        ino: InodeId,
        meta: Metadata,
    ) -> Result<(), AcknoledgeSetAttrError> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "acknowledge_metadata")?;
        let path = arbo.n_get_path_from_inode_id(ino)?;
        let inode = arbo.n_get_inode_mut(ino)?;

        let self_addr =
            LocalConfig::read_lock(&self.network_interface.local_config, "pull_file_sync")
                .expect("pull_fyle_sync: can't get self_addr")
                .general
                .address
                .clone();

        if meta.size != inode.meta.size || meta.perm != inode.meta.perm {
            match &inode.entry {
                FsEntry::Directory(_) if meta.size != inode.meta.size => {
                    return Err(AcknoledgeSetAttrError::WhError {
                        source: WhError::InodeIsADirectory,
                    });
                }
                FsEntry::Directory(_) => {
                    if meta.perm != inode.meta.perm {
                        self.disk
                            .set_permisions(&path, meta.perm as u16)
                            .map_err(|io| AcknoledgeSetAttrError::SetFileSizeIoError { io })?;
                    }
                }
                FsEntry::File(hosts) => {
                    if hosts.contains(&self_addr) {
                        if meta.size != inode.meta.size {
                            self.disk
                                .set_file_size(&path, meta.size as usize)
                                .map_err(|io| AcknoledgeSetAttrError::SetFileSizeIoError { io })?;
                        }
                        if meta.perm != inode.meta.perm {
                            self.disk
                                .set_permisions(&path, meta.perm as u16)
                                .map_err(|io| AcknoledgeSetAttrError::SetFileSizeIoError { io })?;
                        }
                    }
                }
            }
        }

        arbo.n_get_inode_mut(ino)?.meta = meta;
        Ok(())
    }

    pub fn setattr(
        &self,
        ino: InodeId,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<std::time::SystemTime>,
        mtime: Option<std::time::SystemTime>,
        ctime: Option<std::time::SystemTime>,
        file_handle: Option<UUID>,
        flags: Option<u32>,
    ) -> Result<Metadata, SetAttrError> {
        let arbo = Arbo::n_read_lock(&self.arbo, "setattr")?;
        let path = arbo.n_get_path_from_inode_id(ino)?;
        let mut meta = arbo.n_get_inode(ino)?.meta.clone();
        drop(arbo);

        //Except for size, No permissions are required on the file itself, but permission is required on all of the directories in pathname that lead to the file.

        // Extract specific informations from the file handle if it's defined
        let (fh_perm, no_atime) = if let Some(file_handle) = file_handle {
            let file_handler = FileHandleManager::read_lock(&self.file_handles, "setattr")?;
            match file_handler.handles.get(&file_handle) {
                None => return Err(SetAttrError::InvalidFileHandle),
                Some(file_handle) => (Some(file_handle.perm), file_handle.no_atime),
            }
        } else {
            (None, false)
        };

        if let Some(mode) = mode {
            self.disk
                .set_permisions(&path, mode as u16)
                .map_err(|io| SetAttrError::SetPermIoError { io })?;
            meta.perm = mode as u16;
        }
        // Set size if size it's defined, take permission from the file handle if the
        if let Some(size) = size {
            match fh_perm {
                Some(perm) if perm != AccessMode::Write && perm != AccessMode::ReadWrite => {
                    return Err(SetAttrError::SizeNoPerm)
                }
                None if !has_write_perm(meta.perm) => return Err(SetAttrError::SizeNoPerm),
                _ => {
                    // In theory if size > meta.size, the file doesn't change in the memory but in case of read, the read should zero fill the rest of the file
                    // But for now we don't support sparse file
                    self.disk
                        .set_file_size(&path, size as usize)
                        .map_err(|io| SetAttrError::SetFileSizeIoError { io })?;
                    meta.size = size;
                    meta.blocks = (size + BLOCK_SIZE - 1) / BLOCK_SIZE;
                }
            };
        }

        if !no_atime {
            if let Some(atime) = atime {
                meta.atime = atime;
            } else {
                meta.atime = SystemTime::now();
            }
        }

        if let Some(mtime) = mtime {
            meta.mtime = mtime;
        // Only size change represent a modification
        } else if size.is_some() {
            meta.mtime = SystemTime::now();
        }

        meta.ctime = ctime.unwrap_or(SystemTime::now());

        //crtime is ignored because crtime is macos only and should'nt be updated after file creation anyway
        //
        // REVIEW- we could implement this code for a perfect macos 1 to 1, but I think allowing such a feature
        // on only one os is a very weird behavior
        //
        // if cfg!(target_os = "macos") {
        //     if let Some(crtime) = crtime {
        //         meta.crtime = crtime;
        //     }
        // }

        if let Some(uid) = uid {
            meta.uid = uid;
        }
        if let Some(gid) = gid {
            meta.gid = gid;
        }
        if let Some(flags) = flags {
            meta.flags = flags;
        }
        self.network_interface.update_metadata(ino, meta.clone())?;
        return Ok(meta);
    }
}
