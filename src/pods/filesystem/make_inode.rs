use custom_error::custom_error;

use crate::{
    error::WhError,
    pods::{
        filesystem::{File, diffs::{Sig, Signature}, permissions::has_write_perm},
        itree::{EntrySymlink, FsEntry, ITree, Inode},
        whpath::InodeName,
    },
};

use super::{
    file_handle::{AccessMode, FileHandleManager, OpenFlags, UUID},
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
    PermissionDenied = "Permission Denied",
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
        name: InodeName,
        kind: SimpleFileType,
        flags: OpenFlags,
        access: AccessMode,
        permissions: u16,
    ) -> Result<(Inode, UUID), CreateError> {
        let inode = self.make_inode(parent_ino, name, permissions, kind)?;

        let perm = check_permissions(flags, access, inode.meta.perm)?;

        let sig = if matches!(access, AccessMode::Write) {
            Signature::new(&File::empty()).ok()
        } else {
            None
        };

        //TRUNC has no use on a new file so it can be removed

        // CREATE FLAG is set can be on but it has no use for us currently
        //if flags & libc::O_CREAT != 0 {
        //}

        let mut file_handles = FileHandleManager::write_lock(&self.file_handles, "create")?;
        let file_handle = file_handles.insert_new_file_handle(flags, perm, inode.id, sig)?;
        Ok((inode, file_handle))
    }

    /// Create a new empty [Inode], define its informations and register both
    /// in the network and in the local filesystem
    /// Immediately replicated to other peers
    pub fn make_inode(
        &self,
        parent_ino: u64,
        name: InodeName,
        permissions: u16,
        kind: SimpleFileType,
    ) -> Result<Inode, MakeInodeError> {
        let new_entry = match kind {
            SimpleFileType::File => FsEntry::File(vec![self.network_interface.hostname()?.clone()]),
            SimpleFileType::Directory => FsEntry::Directory(Vec::new()),
            SimpleFileType::Symlink => FsEntry::Symlink(EntrySymlink::default()),
        };

        let special_ino = ITree::get_special(name.as_ref(), parent_ino);
        if special_ino.is_some() && kind != SimpleFileType::File {
            return Err(MakeInodeError::ProtectedNameIsFolder);
        }
        let new_inode_id = special_ino
            .ok_or(())
            .or_else(|_| self.network_interface.get_next_inode())?;

        let new_inode = Inode::new(name, parent_ino, new_inode_id, new_entry, permissions);

        let new_path = {
            let itree = ITree::read_lock(&self.itree, "make inode")?;

            let parent = itree.get_inode(parent_ino)?;

            if !has_write_perm(parent.meta.perm) || !has_write_perm(parent.meta.perm) {
                return Err(MakeInodeError::PermissionDenied);
            }
            //check if already exist
            match itree.get_inode_child_by_name(&parent, new_inode.name.as_ref()) {
                Ok(_) => return Err(MakeInodeError::AlreadyExist),
                Err(WhError::InodeNotFound) => {}
                Err(err) => return Err(MakeInodeError::WhError { source: err }),
            }
            let mut new_path = itree.get_path_from_inode_id(parent_ino)?;
            new_path.push((&new_inode.name).into());
            new_path
        };

        match kind {
            SimpleFileType::File => self
                .disk
                .new_file(&new_path, new_inode.meta.perm)
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
