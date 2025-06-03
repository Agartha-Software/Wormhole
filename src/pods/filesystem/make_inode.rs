use custom_error::custom_error;

use crate::{
    config::{types::Config, LocalConfig},
    error::WhError,
    pods::arbo::{
        Arbo, FsEntry, Inode, ARBO_FILE_FNAME, ARBO_FILE_INO, GLOBAL_CONFIG_FNAME,
        GLOBAL_CONFIG_INO, LOCAL_CONFIG_FNAME, LOCAL_CONFIG_INO,
    },
};

use super::{
    file_handle::{FileHandleManager, UUID},
    fs_interface::{FsInterface, SimpleFileType},
    open::{check_permissions, OpenError},
};

custom_error! {pub MakeInodeError
    WhError{source: WhError} = "{source}",
    AlreadyExist = "File already existing",
    ParentNotFound = "Parent does not exist",
    ParentNotFolder = "Parent isn't a folder",
    LocalCreationFailed{io: std::io::Error} = "Local creation failed: {io}",
    ProtectedNameIsFolder = "Protected name can't be used for folders",
}

custom_error! {pub CreateError
    MakeInode{source: MakeInodeError} = "{source}",
    OpenError{source: OpenError} = "{source}",
    WhError{source: WhError} = "{source}",
}

impl FsInterface {
    pub fn create(
        &self,
        parent_ino: u64,
        name: String,
        flags: i32,
    ) -> Result<(Inode, UUID), CreateError> {
        let inode = self.make_inode(parent_ino, name, SimpleFileType::File)?;

        let perm = check_permissions(flags, inode.meta.perm)?;

        //TRUNC has no use on a new file so it can be removed

        // CREATE FLAG is set can be on but it has no use for us currently
        //if flags & libc::O_CREAT != 0 {
        //}

        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "create")?;
        let file_handle = file_handles.insert_new_file_handle(flags, perm)?;
        return Ok((inode, file_handle));
    }

    #[must_use]
    /// Create a new empty [Inode], define its informations and register both
    /// in the network and in the local filesystem
    pub fn make_inode(
        &self,
        parent_ino: u64,
        name: String,
        kind: SimpleFileType,
    ) -> Result<Inode, MakeInodeError> {
        let new_entry = match kind {
            SimpleFileType::File => FsEntry::File(vec![LocalConfig::read_lock(
                &self.network_interface.local_config,
                "remove_inode_locally",
            )?
            .general
            .address
            .clone()]),
            SimpleFileType::Directory => FsEntry::Directory(Vec::new()),
        };

        let special_ino = Arbo::get_special(&name, parent_ino);
        if special_ino.is_some() && kind != SimpleFileType::File {
            return Err(MakeInodeError::ProtectedNameIsFolder);
        }
        let new_inode_id = special_ino
            .ok_or(())
            .or_else(|_| self.network_interface.n_get_next_inode())?;

        let new_inode = Inode::new(name.clone(), parent_ino, new_inode_id, new_entry);

        let mut new_path;
        {
            let arbo = Arbo::n_read_lock(&self.arbo, "make inode")?;

            let parent = arbo.n_get_inode(new_inode.parent)?;
            //check if already exist
            match arbo.n_get_inode_child_by_name(&parent, &new_inode.name) {
                Ok(_) => return Err(MakeInodeError::AlreadyExist),
                Err(WhError::InodeNotFound) => {}
                Err(err) => return Err(MakeInodeError::WhError { source: err }),
            }
            new_path = arbo.n_get_path_from_inode_id(parent_ino)?;
            new_path.push(&name);
        }

        match kind {
            SimpleFileType::File => self
                .disk
                .new_file(&new_path, new_inode.meta.perm)
                .map(|_| ())
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
            SimpleFileType::Directory => self
                .disk
                .new_dir(&new_path, new_inode.meta.perm)
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
        }?;

        self.network_interface
            .register_new_inode(new_inode.clone())?;
        Ok(new_inode)
    }
}
