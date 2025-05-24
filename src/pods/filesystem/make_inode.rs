use custom_error::custom_error;

use crate::{
    config::{types::Config, LocalConfig}, error::WhError, pods::arbo::{Arbo, FsEntry, Inode, ARBO_FILE_FNAME, ARBO_FILE_INO, GLOBAL_CONFIG_FNAME, GLOBAL_CONFIG_INO, LOCAL_CONFIG_FNAME, LOCAL_CONFIG_INO}
};

use super::fs_interface::{FsInterface, SimpleFileType};

custom_error! {pub MakeInode
    WhError{source: WhError} = "{source}",
    AlreadyExist = "File already existing",
    ParentNotFound = "Parent does not exist",
    ParentNotFolder = "Parent isn't a folder",
    LocalCreationFailed{io: std::io::Error} = "Local creation failed: {io}"
}

impl FsInterface {
    #[must_use]
    /// Create a new empty [Inode], define his informations and register both
    /// in the network and in the local filesystem
    pub fn make_inode(
        &self,
        parent_ino: u64,
        name: String,
        kind: SimpleFileType,
    ) -> Result<Inode, MakeInode> {
        let new_entry = match kind {
            SimpleFileType::File => FsEntry::File(vec![LocalConfig::read_lock(&self.network_interface.local_config, "remove_inode_locally")?.general.address.clone()]),
            SimpleFileType::Directory => FsEntry::Directory(Vec::new()),
        };

        let new_inode_id = match (name.as_str(), parent_ino) {
            (GLOBAL_CONFIG_FNAME, 1) => GLOBAL_CONFIG_INO,
            (LOCAL_CONFIG_FNAME, 1) => LOCAL_CONFIG_INO,
            (ARBO_FILE_FNAME, 1) => ARBO_FILE_INO,
            _ => self.network_interface.n_get_next_inode()?,
        };

        let new_inode = Inode::new(name.clone(), parent_ino, new_inode_id, new_entry);

        let mut new_path;
        {
            let arbo = Arbo::n_read_lock(&self.arbo, "make inode")?;

            let parent = arbo.n_get_inode(new_inode.parent)?;
            //check if already exist
            match arbo.n_get_inode_child_by_name(&parent, &new_inode.name) {
                Ok(_) => return Err(MakeInode::AlreadyExist),
                Err(WhError::InodeNotFound) => {}
                Err(err) => return Err(MakeInode::WhError { source: err }),
            }
            new_path = arbo.n_get_path_from_inode_id(parent_ino)?;
            new_path.push(&name);
        }

        match kind {
            SimpleFileType::File => self
                .disk
                .new_file(new_path, new_inode.meta.perm)
                .map(|_| ())
                .map_err(|io| MakeInode::LocalCreationFailed { io }),
            SimpleFileType::Directory => self
                .disk
                .new_dir(new_path, new_inode.meta.perm)
                .map_err(|io| MakeInode::LocalCreationFailed { io }),
        }?;

        self.network_interface
            .register_new_inode(new_inode.clone())?;
        Ok(new_inode)
    }
}
