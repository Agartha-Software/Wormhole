use crate::error::WhResult;

use super::{
    file_handle::{FileHandleManager, UUID},
    fs_interface::FsInterface,
};

impl FsInterface {
    pub fn release(&self, file_handle: UUID) -> WhResult<()> {
        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "release")?;
        file_handles.handles.remove(&file_handle);
        return Ok(());
    }
}
