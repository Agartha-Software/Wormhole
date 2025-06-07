use crate::{
    error::WhError,
    pods::arbo::{Arbo, InodeId},
};
use custom_error::custom_error;
use parking_lot::RwLockReadGuard;

use super::{
    file_handle::{AccessMode, FileHandle, FileHandleManager, UUID},
    fs_interface::FsInterface,
};

custom_error! {
    /// Error describing the write syscall
    pub WriteError
    WhError{source: WhError} = "{source}",
    LocalWriteFailed{io: std::io::Error} = "Local write failed: {io}",
    NoFileHandle = "The file doesn't have a file handle",
    NoWritePermission = "The permissions doesn't allow to write",
}

fn check_file_handle<'a>(
    file_handles: &'a RwLockReadGuard<FileHandleManager>,
    file_handle_id: UUID,
) -> Result<&'a FileHandle, WriteError> {
    match file_handles.handles.get(&file_handle_id) {
        Some(&FileHandle {
            perm: AccessMode::Read,
            direct: _,
            no_atime: _,
        }) => return Err(WriteError::NoWritePermission),
        Some(&FileHandle {
            perm: AccessMode::Execute,
            direct: _,
            no_atime: _,
        }) => return Err(WriteError::NoWritePermission),
        None => return Err(WriteError::NoFileHandle),
        Some(file_handle) => Ok(file_handle),
    }
}

impl FsInterface {
    pub fn write(
        &self,
        id: InodeId,
        data: &[u8],
        offset: usize,
        file_handle: UUID,
    ) -> Result<usize, WriteError> {
        let file_handles = FileHandleManager::read_lock(&self.file_handles, "write")?;
        let _file_handle = check_file_handle(&file_handles, file_handle)?;

        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface.write")?;
        let path = arbo.n_get_path_from_inode_id(id)?;
        drop(arbo);

        let new_size = offset + data.len();
        let written = self
            .disk
            .write_file(&path, data, offset)
            .map_err(|io| WriteError::LocalWriteFailed { io })?;

        self.network_interface.write_file(id, new_size)?;
        Ok(written)
    }
}
