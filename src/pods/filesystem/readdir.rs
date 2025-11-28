use custom_error::custom_error;

use crate::error::{WhError, WhResult};
use crate::pods::arbo::{Arbo, FsEntry, Ino, Inode};
use crate::pods::filesystem::fs_interface::FsInterface;
use crate::pods::filesystem::permissions::has_read_perm;

custom_error! {
    pub ReadDirError
    PermissionError = "No permission to read",
    WhError{ source: WhError} = "{source}",
}

impl FsInterface {
    pub fn read_dir(&self, ino: Ino) -> Result<Vec<Inode>, ReadDirError> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface.read_dir")?;
        let mut dir = arbo.n_get_inode(ino)?.clone();

        if !has_read_perm(dir.meta.perm) {
            return Err(ReadDirError::PermissionError);
        }

        let children = match &dir.entry {
            FsEntry::Directory(children) => children
                .iter()
                .map(|entry| arbo.n_get_inode(*entry).map(|inode| inode.clone()))
                .collect::<WhResult<Vec<Inode>>>(),
            _ => Err(WhError::InodeIsNotADirectory),
        }?;

        let mut links: Vec<Inode> = Vec::with_capacity(children.len() + 2);
        let mut parent = arbo.n_get_inode(dir.parent)?.clone();

        dir.name = ".".to_owned().try_into().unwrap();
        links.push(dir);
        parent.name = "..".to_owned().try_into().unwrap();
        links.push(parent.clone());
        links.extend(children);
        Ok(links)
    }
}
