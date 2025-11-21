use std::{sync::Arc, time::SystemTime};

use crate::{
    error::{WhError, WhResult},
    pods::arbo::{Arbo, InodeId, Metadata, BLOCK_SIZE},
};
use custom_error::custom_error;
use parking_lot::RwLockWriteGuard;

use super::{
    file_handle::{AccessMode, FileHandle, FileHandleManager, UUID},
    fs_interface::FsInterface,
};

custom_error! {
    /// Error describing the write syscall
    #[derive(Clone)]
    pub WriteError
    WhError{source: WhError} = "{source}",
    LocalWriteFailed{io: Arc<std::io::Error>} = "Local write failed: {io}",
    NoFileHandle = "The file doesn't have a file handle",
    NoWritePermission = "The permissions doesn't allow to write",
}

impl From<std::io::Error> for WriteError {
    fn from(io: std::io::Error) -> Self {
        Self::LocalWriteFailed { io: Arc::new(io) }
    }
}

fn check_file_handle<'a>(
    file_handles: &'a mut RwLockWriteGuard<FileHandleManager>,
    file_handle_id: UUID,
) -> Result<&'a mut FileHandle, WriteError> {
    match file_handles.handles.get_mut(&file_handle_id) {
        Some(&mut FileHandle {
            perm: AccessMode::Read,
            direct: _,
            no_atime: _,
            dirty: _,
            ino: _,
            signature: _,
        }) => Err(WriteError::NoWritePermission),
        Some(&mut FileHandle {
            perm: AccessMode::Execute,
            direct: _,
            no_atime: _,
            dirty: _,
            ino: _,
            signature: _,
        }) => Err(WriteError::NoWritePermission),
        None => Err(WriteError::NoFileHandle),
        Some(file_handle) => Ok(file_handle),
    }
}

impl FsInterface {
    /// modifies the local file on disk
    /// marks the file handle as dirty, but does not immediately send the change to other peers
    pub fn write(
        &self,
        id: InodeId,
        data: &[u8],
        offset: usize,
        file_handle: UUID,
    ) -> Result<usize, WriteError> {
        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "write")?;
        let file_handle = check_file_handle(&mut file_handles, file_handle)?;

        file_handle.dirty = true;

        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface.write")?;
        let path = arbo.n_get_path_from_inode_id(id)?;
        drop(arbo);

        let new_size = offset + data.len();
        let written = self.disk.write_file(&path, data, offset)?;

        self.affect_write_locally(id, new_size)?;
        Ok(written)
    }

    fn affect_write_locally(&self, id: InodeId, new_size: usize) -> WhResult<Metadata> {
        let mut arbo = Arbo::n_write_lock(&self.arbo, "network_interface.affect_write_locally")?;
        let inode = arbo.n_get_inode_mut(id)?;
        let new_size = (new_size as u64).max(inode.meta.size);
        inode.meta.size = new_size;
        inode.meta.blocks = new_size.div_ceil(BLOCK_SIZE);

        inode.meta.mtime = SystemTime::now();

        Ok(inode.meta.clone())
    }
}
