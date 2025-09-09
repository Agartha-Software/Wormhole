use crate::pods::{
    arbo::{Arbo, InodeId},
    filesystem::{
        file_handle::{AccessMode, FileHandleManager, OpenFlags},
        permissions::{has_execute_perm, has_read_perm, has_write_perm},
    },
};

use crate::error::WhError;

use custom_error::custom_error;

use super::{file_handle::UUID, fs_interface::FsInterface};

custom_error! {pub OpenError
    WhError{source: WhError} = "{source}",
    TruncReadOnly = "You can't truncate a file while opening in read-only",
    WrongPermissions = "Tried to open a file without permission",
    MultipleAccessFlags = "Multiple access flags given",
}

const FMODE_EXEC: i32 = 0x20;

impl AccessMode {
    #[cfg(target_os = "linux")]
    pub fn from_libc(flags: i32) -> Result<AccessMode, OpenError> {
        match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                if flags & FMODE_EXEC != 0 {
                    Ok(AccessMode::Execute)
                } else {
                    Ok(AccessMode::Read)
                }
            }
            libc::O_WRONLY => Ok(AccessMode::Write),
            libc::O_RDWR => Ok(AccessMode::ReadWrite),
            //Exactly one access mode flag must be specified
            _ => Err(OpenError::MultipleAccessFlags),
        }
    }
}

pub fn check_permissions(
    flags: OpenFlags,
    access: AccessMode,
    inode_perm: u16,
) -> Result<AccessMode, OpenError> {
    match access {
        AccessMode::Void => Ok(AccessMode::Void),
        AccessMode::Read => {
            if !has_read_perm(inode_perm) {
                Err(OpenError::WrongPermissions)
            //Behavior is undefined, but most filesystems return EACCES
            } else if flags.trunc {
                //EACCESS
                Err(OpenError::TruncReadOnly)
            //Open is from internal exec syscall
            } else if flags.exec {
                if !has_execute_perm(inode_perm) {
                    Err(OpenError::WrongPermissions)
                } else {
                    Ok(AccessMode::Execute)
                }
            } else {
                Ok(AccessMode::Read)
            }
        }
        AccessMode::Write if !has_write_perm(inode_perm) => Err(OpenError::WrongPermissions),
        AccessMode::Write => Ok(AccessMode::Write),
        AccessMode::ReadWrite if !has_read_perm(inode_perm) || !has_write_perm(inode_perm) => {
            Err(OpenError::WrongPermissions)
        }
        AccessMode::ReadWrite => Ok(AccessMode::ReadWrite),
        AccessMode::Execute => {
            if has_read_perm(inode_perm) {
                Err(OpenError::WrongPermissions)
            //Behavior is undefined, but most filesystems return EACCES
            } else {
                if !has_execute_perm(inode_perm) {
                    Err(OpenError::WrongPermissions)
                } else {
                    Ok(AccessMode::Execute)
                }
            }
        }
    }
}

impl FsInterface {
    pub fn open(
        &self,
        ino: InodeId,
        flags: OpenFlags,
        access: AccessMode,
    ) -> Result<UUID, OpenError> {
        let inode_perm = Arbo::n_read_lock(&self.arbo, "open")?
            .n_get_inode(ino)?
            .meta
            .perm;

        let perm = check_permissions(flags, access, inode_perm)?;

        if flags.trunc {
            //TODO: Trunc over the network
        }

        // libc::O_CREAT is never set, The flag is set only with the create syscall

        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "open")?;
        file_handles
            .insert_new_file_handle(flags, perm, ino)
            .map_err(|err| err.into())
    }
}
