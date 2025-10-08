use crate::error::WhResult;

use super::{
    file_handle::{FileHandleManager, UUID},
    fs_interface::FsInterface,
};

impl FsInterface {
    pub fn release(&self, file_handle: UUID) -> WhResult<()> {
        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "release")?;
        if let Some(handle) = file_handles.handles.remove(&file_handle) {
            if handle.dirty {
                self.network_interface.apply_redundancy(handle.ino);
            }
        }
        return Ok(());
    }
}
