use std::io;

use custom_error::custom_error;

use crate::{
    error::{WhError, WhResult},
    pods::{
        arbo::{Arbo, InodeId, Metadata},
        filesystem::flush::FlushError,
        filesystem::permissions::has_write_perm,
        whpath::WhPath,
    },
};

use super::{
    fs_interface::FsInterface, make_inode::MakeInodeError, read::ReadError,
    remove_inode::RemoveFileError,
};

custom_error! {
    /// Error describing the removal of a [Inode] from the [Arbo] and the local file or folder
    pub RenameError
    WhError{source: WhError} = "{source}",
    OverwriteNonEmpty = "Can't overwrite non-empty dir",
    LocalOverwriteFailed{io: std::io::Error} = "Local Overwriting failed: {io}",
    SourceParentNotFound = "Source parent does not exist",
    SourceParentNotFolder = "Source parent isn't a folder",
    DestinationParentNotFound = "Destination parent does not exist",
    DestinationParentNotFolder = "Destination parent isn't a folder",
    DestinationExists = "Destination name already exists",
    LocalRenamingFailed{io: std::io::Error} = "Local renaming failed: {io}",
    ProtectedNameIsFolder = "Protected name can't be used for folders",
    ReadFailed{source: ReadError} = "Read failed on copy: {source}",
    LocalWriteFailed{io: std::io::Error} = "Write failed on copy: {io}",
    FlushError{source: FlushError} = "Couldn't flush changes on special: {source}",
    PermissionDenied = "Permission denied"
}

impl FsInterface {
    fn construct_file_path(&self, parent: InodeId, name: &String) -> WhResult<WhPath> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface.get_begin_path_end_path")?;
        let parent_path = arbo.n_get_path_from_inode_id(parent)?;

        Ok(parent_path.join(name))
    }

    fn rename_locally(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: &String,
        new_name: &String,
    ) -> Result<(), RenameError> {
        let parent_path = self.construct_file_path(parent, name)?;
        let new_parent_path = self.construct_file_path(new_parent, new_name)?;

        if self.disk.file_exists(&parent_path) {
            self.disk
                .mv_file(&parent_path, &new_parent_path)
                .map_err(|io| RenameError::LocalRenamingFailed { io })
        } else {
            Ok(())
        }
    }

    pub fn set_meta_size(&self, ino: InodeId, meta: Metadata) -> Result<(), RenameError> {
        let path = Arbo::n_read_lock(&self.arbo, "rename")?.n_get_path_from_inode_id(ino)?;

        self.disk
            .set_file_size(&path, meta.size as usize)
            .map_err(|io| RenameError::LocalOverwriteFailed { io })?;

        self.network_interface.update_metadata(ino, meta)?;
        Ok(())
    }

    ///
    /// handle rename with special files
    /// special files have a special inode, so can't be naively renamed
    /// the source file must be deleted and the destination must be created
    ////
    fn rename_special(
        &self,
        new_parent: InodeId,
        new_name: String,
        source_ino: u64,
        dest_ino: Option<u64>,
    ) -> Result<(), RenameError> {
        let meta = Arbo::n_read_lock(&self.arbo, "fs_interface::remove_inode")?
            .n_get_inode(source_ino)
            .expect("already checked")
            .meta
            .clone();
        let mut data = vec![0; meta.size as usize];
        self.get_file_data_sync(source_ino, 0, &mut data)
            .map_err(|err| match err {
                ReadError::WhError { source } => RenameError::WhError { source },
                err => err.into(),
            })?;

        let dest_ino = if let Some(dest_ino) = dest_ino {
            let mut meta = self
                .n_get_inode_attributes(dest_ino)
                .map_err(|_| WhError::InodeNotFound)?;
            meta.size = 0;
            self.set_meta_size(source_ino, meta)?;
            dest_ino
        } else {
            self.make_inode(new_parent, new_name, meta.perm, meta.kind)
                .map_err(|err| match err {
                    MakeInodeError::WhError { source } => RenameError::WhError { source },
                    MakeInodeError::AlreadyExist => RenameError::DestinationExists,
                    MakeInodeError::ParentNotFound => RenameError::DestinationParentNotFound,
                    MakeInodeError::ParentNotFolder => RenameError::DestinationParentNotFolder,
                    MakeInodeError::LocalCreationFailed { io } => {
                        RenameError::LocalRenamingFailed { io }
                    }
                    MakeInodeError::ProtectedNameIsFolder => RenameError::ProtectedNameIsFolder,
                    MakeInodeError::PermissionDenied => RenameError::LocalRenamingFailed {
                        io: std::io::ErrorKind::PermissionDenied.into(),
                    },
                })?
                .id
        };

        {
            // write the new file
            let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface.write")?;
            let path = arbo.n_get_path_from_inode_id(dest_ino)?;
            drop(arbo);

            self.disk
                .write_file(&path, &data, 0)
                .map_err(|io| RenameError::LocalWriteFailed { io })?;

            self.flush(dest_ino, None)?;
        }
        self.remove_inode(source_ino).map_err(|err| match err {
            RemoveFileError::WhError { source } => RenameError::WhError { source },
            RemoveFileError::NonEmpty => unreachable!("special files cannot be folders"),
            RemoveFileError::LocalDeletionFailed { io } => RenameError::LocalRenamingFailed { io },
            RemoveFileError::PermissionDenied => RenameError::PermissionDenied,
        })?;

        Ok(())
    }

    /// Rename a file, by changing its name but usually not its ino
    ///
    /// overwrite: silently delete a file with the destination name
    ///
    /// special: when using special name, completely switch behavior:
    ///  - if overwrite, delete destination officialy
    ///  - create a new destination inode
    ///  - copy/move  contents
    ///  - delete the source inode
    ///
    /// Immediately replicated to other peers
    pub fn rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: &String,
        new_name: &String,
        overwrite: bool,
    ) -> Result<(), RenameError> {
        if parent == new_parent && name == new_name {
            return Ok(());
        }

        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::remove_inode")?;
        let p_inode = arbo.n_get_inode(parent).map_err(|err| match err {
            WhError::InodeNotFound => RenameError::SourceParentNotFound,
            WhError::InodeIsNotADirectory => RenameError::SourceParentNotFolder,
            source => RenameError::WhError { source },
        })?;
        if !has_write_perm(p_inode.meta.perm) {
            return Err(RenameError::PermissionDenied);
        }
        let src_ino = arbo
            .n_get_inode_child_by_name(p_inode, &name)
            .map_err(|err| match err {
                WhError::InodeNotFound => RenameError::SourceParentNotFound,
                WhError::InodeIsNotADirectory => RenameError::SourceParentNotFolder,
                source => RenameError::WhError { source },
            })?
            .id; // assert source file exists
        let dest_ino = match arbo.n_get_inode_child_by_name(arbo.n_get_inode(new_parent)?, new_name)
        {
            Ok(inode) => Some(inode.id),
            Err(WhError::InodeNotFound) => None,
            Err(source) => return Err(source.into()),
        };
        drop(arbo);

        if dest_ino.is_some() && !overwrite {
            log::debug!("not overwriting!!");
            return Err(RenameError::DestinationExists);
        }
        if Arbo::get_special(name, parent).is_some()
            || Arbo::get_special(new_name, new_parent).is_some()
        {
            return self.rename_special(new_parent, new_name.clone(), src_ino, dest_ino);
        }

        if let Some(dest_ino) = dest_ino {
            log::debug!("overwriting!!");
            self.recept_remove_inode(dest_ino).map_err(|e| match e {
                RemoveFileError::LocalDeletionFailed { io } => {
                    RenameError::LocalOverwriteFailed { io }
                }
                RemoveFileError::NonEmpty => return RenameError::OverwriteNonEmpty,
                RemoveFileError::WhError { source } => return RenameError::WhError { source },
                RemoveFileError::PermissionDenied => RenameError::PermissionDenied,
            })?;
        }

        self.rename_locally(parent, new_parent, name, new_name)?;
        self.network_interface
            .n_rename(parent, new_parent, name, new_name, overwrite)?;
        Ok(())
    }

    pub fn recept_rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: &String,
        new_name: &String,
        overwrite: bool,
    ) -> Result<(), RenameError> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::remove_inode")?;
        let dest_ino = match arbo.n_get_inode_child_by_name(arbo.n_get_inode(new_parent)?, new_name)
        {
            Ok(inode) => Some(inode.id),
            Err(WhError::InodeNotFound) => None,
            Err(source) => return Err(source.into()),
        };
        drop(arbo);
        if let Some(dest_ino) = dest_ino {
            if overwrite {
                log::debug!("overwriting!!");
                self.recept_remove_inode(dest_ino).map_err(|e| match e {
                    RemoveFileError::LocalDeletionFailed { io } => {
                        RenameError::LocalOverwriteFailed { io }
                    }
                    RemoveFileError::NonEmpty => RenameError::OverwriteNonEmpty,
                    RemoveFileError::WhError { source } => RenameError::WhError { source },
                    RemoveFileError::PermissionDenied => RenameError::PermissionDenied,
                })?;
            } else {
                log::debug!("not overwriting!!");
                return Err(RenameError::DestinationExists);
            }
        }
        self.rename_locally(parent, new_parent, name, new_name)
            .or_else(|e| match e {
                RenameError::LocalRenamingFailed { io } if io.kind() == io::ErrorKind::NotFound => {
                    Ok(())
                }
                other => Err(other),
            })?;
        self.network_interface
            .acknowledge_rename(parent, new_parent, name, new_name)?;
        Ok(())
    }
}
