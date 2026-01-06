use custom_error::custom_error;

use crate::error::{WhError, WhResult};
use crate::pods::filesystem::fs_interface::FsInterface;
use crate::pods::filesystem::permissions::has_read_perm;
use crate::pods::itree::{FsEntry, ITree, Ino, Metadata};

custom_error! {
    pub ReadDirError
    PermissionError = "No permission to read",
    WhError{ source: WhError} = "{source}",
}

impl FsInterface {
    pub fn read_dir(&self, ino: Ino) -> Result<Vec<(u64, String, Metadata)>, ReadDirError> {
        let itree = ITree::read_lock(&self.itree, "fs_interface.read_dir")?;
        let dir = itree.get_inode(ino)?.clone();

        if !has_read_perm(dir.meta.perm) {
            return Err(ReadDirError::PermissionError);
        }

        let children = match &dir.entry {
            FsEntry::Directory(children) => children
                .iter()
                .map(|entry| {
                    itree
                        .get_inode(*entry)
                        .map(|inode| (inode.id, inode.name.as_str().to_owned(), inode.meta.clone()))
                })
                .collect::<WhResult<Vec<(u64, String, Metadata)>>>(),
            _ => Err(WhError::InodeIsNotADirectory),
        }?;

        let mut links: Vec<(u64, String, Metadata)> = Vec::with_capacity(children.len() + 2);
        let parent = itree.get_inode(dir.parent)?.clone();

        links.push((dir.id, ".".to_owned(), dir.meta));
        links.push((parent.id, "..".to_owned(), parent.meta));
        links.extend(children);
        Ok(links)
    }
}
