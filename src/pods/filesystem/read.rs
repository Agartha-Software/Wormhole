use crate::pods::arbo::Arbo;
use crate::pods::filesystem::file_handle::{AccessMode, FileHandle, FileHandleManager, UUID};
use crate::pods::network::pull_file::PullError;
use crate::{error::WhError, pods::arbo::InodeId};
use custom_error::custom_error;
use parking_lot::RwLockReadGuard;

use super::fs_interface::FsInterface;

custom_error! {
    /// Error describing the read syscall
    pub ReadError
    WhError{source: WhError} = "{source}",
    PullError{source: PullError} = "{source}",
    LocalReadFailed{io: std::io::Error} = "Local read failed: {io}",
    CantPull = "Unable to pull file",
    NoReadPermission = "The permissions doesn't allow to read",
    NoFileHandle = "The file doesn't have a file handle",
}

fn check_file_handle<'a>(
    file_handles: &'a RwLockReadGuard<FileHandleManager>,
    file_handle_id: UUID,
) -> Result<&'a FileHandle, ReadError> {
    match file_handles.handles.get(&file_handle_id) {
        Some(&FileHandle {
            perm: AccessMode::Write,
            direct: _,
            no_atime: _,
        }) => return Err(ReadError::NoReadPermission),
        Some(&FileHandle {
            perm: AccessMode::Execute,
            direct: _,
            no_atime: _,
        }) => return Err(ReadError::NoReadPermission),
        None => return Err(ReadError::NoFileHandle),
        Some(file_handle) => Ok(file_handle),
    }
}

impl FsInterface {
    pub fn get_file_data(
        &self,
        file: InodeId,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, ReadError> {
        let ok = match self.network_interface.pull_file_sync(file)? {
            None => true,
            Some(call) => self.network_interface.callbacks.n_wait_for(call)?,
        };

        if !ok {
            return Err(ReadError::CantPull);
        }

        self.disk
            .read_file(
                &Arbo::n_read_lock(&self.arbo, "read_file")?.n_get_path_from_inode_id(file)?,
                offset,
                buf,
            )
            .map_err(|io| ReadError::LocalReadFailed { io })
    }

    pub fn read_file(
        &self,
        file: InodeId,
        offset: usize,
        buf: &mut [u8],
        file_handle: UUID,
    ) -> Result<usize, ReadError> {
        {
            let file_handles = FileHandleManager::read_lock(&self.file_handles, "read")?;
            let _file_handle = check_file_handle(&file_handles, file_handle)?;
        }

        self.get_file_data(file, offset, buf)
    }
}
