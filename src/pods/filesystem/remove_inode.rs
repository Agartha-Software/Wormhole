use custom_error::custom_error;

use crate::{
    config::{types::Config, LocalConfig}, error::WhError, pods::arbo::{Arbo, FsEntry, InodeId}
};

use super::fs_interface::FsInterface;

custom_error! {
    /// Error describing the removal of a [Inode] from the [Arbo]
    pub RemoveInodeError
    WhError{source: WhError} = "{source}",
    NonEmpty = "Can't remove non-empty dir",
}

custom_error! {
    /// Error describing the removal of a [Inode] from the [Arbo] and the local file or folder
    pub RemoveFileError
    WhError{source: WhError} = "{source}",
    NonEmpty = "Can't remove non-empty dir",
    LocalDeletionFailed{io: std::io::Error} = "Local Deletion failed: {io}"
}

impl From<RemoveInodeError> for RemoveFileError {
    fn from(value: RemoveInodeError) -> Self {
        match value {
            RemoveInodeError::WhError { source } => RemoveFileError::WhError { source },
            RemoveInodeError::NonEmpty => RemoveFileError::NonEmpty,
        }
    }
}

impl FsInterface {
    // NOTE - system specific (fuse/winfsp) code that need access to arbo or other classes
    pub fn fuse_remove_inode(
        &self,
        parent: InodeId,
        name: &std::ffi::OsStr,
    ) -> Result<(), RemoveFileError> {
        let target = {
            let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::fuse_remove_inode")?;
            let parent = arbo.n_get_inode(parent)?;
            arbo.n_get_inode_child_by_name(parent, &name.to_string_lossy().to_string())?
                .id
        };

        self.remove_inode(target)
    }

    pub fn remove_inode_locally(&self, id: InodeId) -> Result<(), RemoveFileError> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::remove_inode")?;
        let to_remove_path = arbo.n_get_path_from_inode_id(id)?;

        match &arbo.n_get_inode(id)?.entry {
            FsEntry::File(hosts) if hosts.contains(&LocalConfig::read_lock(&self.network_interface.local_config, "remove_inode_locally")?.general.address) => self
                .disk
                .remove_file(&to_remove_path)
                .map_err(|io| RemoveFileError::LocalDeletionFailed { io })?,
            FsEntry::File(_) => {
                // TODO: Remove when wormhole initialisation is cleaner
                // try to delete the file even if it's not owned to prevent from conflicts on creation later
                let _ = self.disk.remove_file(&to_remove_path);
            }
            FsEntry::Directory(children) if children.is_empty() => self
                .disk
                .remove_dir(&to_remove_path)
                .map_err(|io| RemoveFileError::LocalDeletionFailed { io })?,
            FsEntry::Directory(_) => return Err(RemoveFileError::NonEmpty),
        };
        Ok(())
    }

    pub fn remove_inode(&self, id: InodeId) -> Result<(), RemoveFileError> {
        self.remove_inode_locally(id)?;
        self.network_interface.unregister_inode(id)?;
        Ok(())
    }

    pub fn recept_remove_inode(&self, id: InodeId) -> Result<(), RemoveFileError> {
        self.remove_inode_locally(id)?;
        self.network_interface.acknowledge_unregister_inode(id)?;
        Ok(())
    }
}
