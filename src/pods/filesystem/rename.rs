use custom_error::custom_error;

use crate::{
    error::{WhError, WhResult},
    pods::{
        arbo::{Arbo, InodeId, Metadata},
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
}

impl FsInterface {
    fn construct_file_path(&self, parent: InodeId, name: &String) -> WhResult<WhPath> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface.get_begin_path_end_path")?;
        let parent_path = arbo.n_get_path_from_inode_id(parent)?;

        return Ok(parent_path.join(name));
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

        self.disk
            .mv_file(&parent_path, &new_parent_path)
            .map_err(|io| RenameError::LocalRenamingFailed { io })
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
        new_name: &String,
        source_ino: u64,
        dest_ino: Option<u64>,
    ) -> Result<(), RenameError> {
        let meta = Arbo::n_read_lock(&self.arbo, "fs_interface::remove_inode")?
            .n_get_inode(source_ino)
            .expect("already checked")
            .meta
            .clone();
        let mut data = vec![];
        data.resize(meta.size as usize, 0u8);
        self.get_file_data(source_ino, 0, &mut data)
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
            self.make_inode(new_parent, new_name.clone(), meta.perm, meta.kind)
                .map_err(|err| match err {
                    MakeInodeError::WhError { source } => RenameError::WhError { source },
                    MakeInodeError::AlreadyExist => RenameError::DestinationExists,
                    MakeInodeError::ParentNotFound => RenameError::DestinationParentNotFound,
                    MakeInodeError::ParentNotFolder => RenameError::DestinationParentNotFolder,
                    MakeInodeError::LocalCreationFailed { io } => {
                        RenameError::LocalRenamingFailed { io }
                    }
                    MakeInodeError::ProtectedNameIsFolder => RenameError::ProtectedNameIsFolder,
                })?
                .id
        };

        {
            // write the new file
            let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface.write")?;
            let path = arbo.n_get_path_from_inode_id(dest_ino)?;
            drop(arbo);

            let new_size = data.len();
            self.disk
                .write_file(&path, &data, 0)
                .map_err(|io| RenameError::LocalWriteFailed { io })?;

            self.network_interface.write_file(dest_ino, new_size)?;
        }
        self.remove_inode(source_ino).map_err(|err| match err {
            RemoveFileError::WhError { source } => RenameError::WhError { source },
            RemoveFileError::NonEmpty => unreachable!("special files cannot be folders"),
            RemoveFileError::LocalDeletionFailed { io } => RenameError::LocalRenamingFailed { io },
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
        let src_ino = arbo
            .n_get_inode_child_by_name(
                arbo.n_get_inode(parent).map_err(|err| match err {
                    WhError::InodeNotFound => RenameError::SourceParentNotFound,
                    WhError::InodeIsNotADirectory => RenameError::SourceParentNotFolder,
                    source => RenameError::WhError { source },
                })?,
                &name,
            )
            .map_err(|err| match err {
                WhError::InodeNotFound => RenameError::SourceParentNotFound,
                WhError::InodeIsNotADirectory => RenameError::SourceParentNotFolder,
                source => RenameError::WhError { source },
            })?
            .id; // assert source file exists
        let dest_ino =
            match arbo.n_get_inode_child_by_name(arbo.n_get_inode(new_parent)?, &new_name) {
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
            return self.rename_special(new_parent, new_name, src_ino, dest_ino);
        }

        if let Some(dest_ino) = dest_ino {
            log::debug!("overwriting!!");
            match self.recept_remove_inode(dest_ino) {
                Ok(_) => (),
                Err(RemoveFileError::LocalDeletionFailed { io }) => {
                    return Err(RenameError::LocalOverwriteFailed { io })
                }
                Err(RemoveFileError::NonEmpty) => return Err(RenameError::OverwriteNonEmpty),
                Err(RemoveFileError::WhError { source }) => {
                    return Err(RenameError::WhError { source })
                }
            }
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
        let dest_ino =
            match arbo.n_get_inode_child_by_name(arbo.n_get_inode(new_parent)?, &new_name) {
                Ok(inode) => Some(inode.id),
                Err(WhError::InodeNotFound) => None,
                Err(source) => return Err(source.into()),
            };
        drop(arbo);
        if let Some(dest_ino) = dest_ino {
            if overwrite {
                log::debug!("overwriting!!");
                match self.recept_remove_inode(dest_ino) {
                    Ok(_) => (),
                    Err(RemoveFileError::LocalDeletionFailed { io }) => {
                        return Err(RenameError::LocalOverwriteFailed { io })
                    }
                    Err(RemoveFileError::NonEmpty) => return Err(RenameError::OverwriteNonEmpty),
                    Err(RemoveFileError::WhError { source }) => {
                        return Err(RenameError::WhError { source })
                    }
                }
            } else {
                log::debug!("not overwriting!!");
                return Err(RenameError::DestinationExists);
            }
        }
        self.network_interface
            .acknowledge_rename(parent, new_parent, name, new_name)?;
        Ok(())
    }
}
