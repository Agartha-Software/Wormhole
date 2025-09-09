use crate::{error::WhResult, pods::arbo::InodeId};

use super::{
    file_handle::{FileHandleManager, UUID},
    fs_interface::FsInterface,
};

impl FsInterface {
    pub fn release(&self, file_handle: UUID, ino: InodeId) -> WhResult<()> {
        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "release")?;
        if let Some(handle) = file_handles.handles.remove(&file_handle) {
            if handle.dirty {
                self.network_interface.apply_redundancy(handle.ino);
            }
        }
        return Ok(());
    }
}
