use crate::{error::WhError, pods::filesystem::flush::FlushError};

use super::{
    file_handle::{FileHandleManager, UUID},
    fs_interface::FsInterface,
};

impl FsInterface {
    pub fn release(&self, file_handle: UUID) -> Result<(), FlushError> {
        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "release")?;
        if let Some(mut handle) = file_handles.handles.remove(&file_handle) {
            if handle.dirty {
                self.flush(handle.ino, Some(&mut handle))?;
                self.network_interface.apply_redundancy(handle.ino);
            }
        } else {
            return Err(WhError::InodeNotFound.into());
        }
        Ok(())
    }
}
