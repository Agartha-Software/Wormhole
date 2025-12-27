#[cfg(target_os = "linux")]
use crate::pods::{filesystem::permissions::has_write_perm, whpath::InodeName};

use crate::{
    error::WhError,
    pods::itree::{FsEntry, ITree, Ino},
};
use custom_error::custom_error;

use super::fs_interface::FsInterface;

custom_error! {
    /// Error describing the removal of a [Inode] from the [ITree]
    pub RemoveInodeError
    WhError{source: WhError} = "{source}",
    NonEmpty = "Can't remove non-empty dir",
}

custom_error! {
    /// Error describing the removal of a [Inode] from the [ITree] and the local file or folder
    pub RemoveFileError
    WhError{source: WhError} = "{source}",
    NonEmpty = "Can't remove non-empty dir",
    LocalDeletionFailed{io: std::io::Error} = "Local Deletion failed: {io}",
    PermissionDenied  = "Permission denied",
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
    // NOTE - system specific (fuse/winfsp) code that need access to itree or other classes
    #[cfg(target_os = "linux")]
    pub fn fuse_remove_inode(&self, parent: Ino, name: InodeName) -> Result<(), RemoveFileError> {
        let target = {
            let itree = ITree::n_read_lock(&self.itree, "fs_interface::fuse_remove_inode")?;
            let parent = itree.n_get_inode(parent)?;
            if !has_write_perm(parent.meta.perm) {
                return Err(RemoveFileError::PermissionDenied);
            }
            itree.n_get_inode_child_by_name(parent, name.as_ref())?.id
        };

        self.remove_inode(target)
    }

    pub fn remove_inode_locally(&self, id: Ino) -> Result<(), RemoveFileError> {
        let itree = ITree::n_read_lock(&self.itree, "fs_interface::remove_inode")?;
        let to_remove_path = itree.n_get_path_from_inode_id(id)?;
        let entry = itree.n_get_inode(id)?.entry.to_owned();
        drop(itree);

        match entry {
            FsEntry::File(hosts) if hosts.contains(&self.network_interface.hostname()?) => self
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
            #[cfg(target_os = "linux")]
            FsEntry::Directory(_) => return Err(RemoveFileError::NonEmpty),
            #[cfg(target_os = "windows")]
            FsEntry::Directory(children) => {
                children.iter().try_for_each(|c| self.remove_inode(*c))?
            }
        };
        Ok(())
    }

    /// Remove an [Inode] from the ITree
    /// Immediately replicated to other peers
    pub fn remove_inode(&self, id: Ino) -> Result<(), RemoveFileError> {
        self.remove_inode_locally(id)?;
        self.network_interface.unregister_inode(id)?;
        Ok(())
    }

    pub fn recept_remove_inode(&self, id: Ino) -> Result<(), RemoveFileError> {
        self.remove_inode_locally(id)?;
        self.network_interface.acknowledge_unregister_inode(id)?;
        Ok(())
    }
}
