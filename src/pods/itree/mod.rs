pub mod creation;
mod fsentry;
mod inode;

pub use fsentry::*;
pub use inode::*;
use libp2p::PeerId;

#[cfg(target_os = "windows")]
pub use crate::pods::itree::WINDOWS_DEFAULT_PERMS_MODE;
use crate::{
    data::tree_hosts::TreeLine,
    error::WhResult,
    pods::whpath::{InodeName, InodeNameError, WhPath},
};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    ops::RangeFrom,
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime},
};

#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

use crate::error::WhError;
use crate::pods::filesystem::fs_interface::SimpleFileType;

use crate::pods::filesystem::{make_inode::MakeInodeError, remove_inode::RemoveInodeError};

// SECTION consts

pub const LOCK_TIMEOUT: Duration = Duration::new(5, 0);

// !SECTION

pub const GLOBAL_CONFIG_INO: u64 = 2;
pub const GLOBAL_CONFIG_FNAME: &str = ".global_config.toml";
pub const LOCAL_CONFIG_INO: u64 = 3;
pub const LOCAL_CONFIG_FNAME: &str = ".local_config.toml";
pub const ITREE_FILE_INO: u64 = 4;
pub const ITREE_FILE_FNAME: &str = ".itree";
pub const FIRST_INO: u64 = 11;

// SECTION types
pub type ITreeIndex = HashMap<Ino, Inode>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ITree {
    pub entries: ITreeIndex,
    pub next_ino: RangeFrom<Ino>,
}

pub const BLOCK_SIZE: u64 = 512;

// !SECTION

impl ITree {
    pub fn new() -> Self {
        let mut itree: Self = Self {
            entries: HashMap::new(),
            next_ino: FIRST_INO..,
        };

        itree.entries.insert(
            ROOT,
            Inode {
                parent: ROOT,
                id: ROOT,
                name: InodeName::root(),
                entry: FsEntry::Directory(vec![]),
                meta: Metadata {
                    ino: ROOT,
                    size: 0,
                    blocks: 0,
                    atime: SystemTime::now(),
                    mtime: SystemTime::now(),
                    ctime: SystemTime::now(),
                    crtime: SystemTime::now(),
                    kind: SimpleFileType::Directory,
                    perm: 0o755,
                    nlink: 2, // Start with 2, one for this link (`self/`) and one for self-referential (`self/.`)
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    blksize: 1,
                    flags: 0,
                },
                xattrs: HashMap::new(),
            },
        );
        itree
    }

    /// Reserve an Ino
    /// when this function returns, the Ino is permanently reserved
    pub fn reserve_ino(&mut self) -> WhResult<Ino> {
        self.next_ino
            .next()
            .ok_or(WhError::WouldBlock {
                called_from: "reserve_ino".to_owned(),
            })
            .inspect_err(|_| log::error!("Ran out of Ino, returning Wh::WouldBlock"))
    }

    /// Mark Ino-s already reserved
    /// This is for example if a peer reserves an Ino and we need to catch up
    /// to not give this ino out again
    pub fn mark_reserved_ino(&mut self, new: Ino) -> WhResult<()> {
        let next = &mut self.next_ino;

        if next.start > new {
            return Err(WhError::WouldBlock {
                called_from: "mark_reserved_ino: new is less than current".to_owned(),
            });
        }
        *next = new + 1..;
        Ok(())
    }

    /// Removed 'local only' files from the tree
    /// This takes and mutates self, so that it can't accidentally be used in place
    /// only meant to create a 'clean' network copy of an itree for sharing
    pub fn clean_local(mut self) -> Self {
        self.entries.retain(|ino, _| !ITree::is_local_only(*ino));
        self.entries.entry(1u64).and_modify(|inode| {
            if let FsEntry::Directory(childrens) = &mut inode.entry {
                childrens.retain(|x| !ITree::is_local_only(*x));
            }
        });
        self
    }

    pub fn raw_entries(&self) -> &ITreeIndex {
        &self.entries
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, Ino, Inode> {
        self.entries.iter()
    }

    // Use only if you know what you're doing, as those modifications won't be propagated to the network
    pub fn inodes_mut(&mut self) -> std::collections::hash_map::ValuesMut<'_, Ino, Inode> {
        self.entries.values_mut()
    }

    pub fn get_special(name: &str, parent_ino: u64) -> Option<u64> {
        match (name, parent_ino) {
            (GLOBAL_CONFIG_FNAME, 1) => Some(GLOBAL_CONFIG_INO),
            (LOCAL_CONFIG_FNAME, 1) => Some(LOCAL_CONFIG_INO),
            _ => None,
        }
    }

    pub fn is_special(ino: u64) -> bool {
        ino <= 10u64
    }

    pub fn is_local_only(ino: u64) -> bool {
        ino == LOCAL_CONFIG_INO // ".local_config.toml"
    }

    pub fn read_lock<'a>(
        itree: &'a Arc<RwLock<ITree>>,
        called_from: &'a str,
    ) -> WhResult<RwLockReadGuard<'a, ITree>> {
        itree.try_read_for(LOCK_TIMEOUT).ok_or(WhError::WouldBlock {
            called_from: called_from.to_owned(),
        })
    }

    pub fn write_lock<'a>(
        itree: &'a Arc<RwLock<ITree>>,
        called_from: &'a str,
    ) -> WhResult<RwLockWriteGuard<'a, ITree>> {
        itree
            .try_write_for(LOCK_TIMEOUT)
            .ok_or(WhError::WouldBlock {
                called_from: called_from.to_owned(),
            })
    }

    pub fn files_hosted_only_by<'a>(
        &'a self,
        host: &'a PeerId,
    ) -> impl Iterator<Item = &'a Inode> + use<'a> {
        self.iter()
            .filter_map(move |(_, inode)| match &inode.entry {
                FsEntry::File(hosts) => {
                    if hosts.len() == 1 && hosts.contains(host) {
                        Some(inode)
                    } else {
                        None
                    }
                }
                _ => None,
            })
    }

    /// Insert a given [Inode] inside the local itree
    pub fn add_inode(&mut self, inode: Inode) -> Result<(), MakeInodeError> {
        if self.entries.contains_key(&inode.id) {
            return Err(MakeInodeError::AlreadyExist);
        }

        match self.entries.get_mut(&inode.parent) {
            None => Err(MakeInodeError::ParentNotFound),
            Some(Inode {
                parent: _,
                id: _,
                name: _,
                entry: FsEntry::Directory(parent_children),
                meta: _,
                xattrs: _,
            }) => {
                parent_children.push(inode.id);
                self.entries.insert(inode.id, inode);
                Ok(())
            }
            Some(_) => Err(MakeInodeError::ParentNotFolder),
        }
    }

    /// Create a new [Inode] from the given parameters and insert it inside the local itree
    pub fn add_inode_from_parameters(
        &mut self,
        name: InodeName,
        id: Ino,
        parent_ino: Ino,
        entry: FsEntry,
        perm: u16,
    ) -> Result<(), MakeInodeError> {
        let inode = Inode::new(name, parent_ino, id, entry, perm);

        self.add_inode(inode)
    }

    pub fn remove_child(&mut self, parent: Ino, child: Ino) -> WhResult<()> {
        let parent = self.get_inode_mut(parent)?;

        let children = match &mut parent.entry {
            FsEntry::Directory(children) => children,
            _ => return Err(WhError::InodeIsNotADirectory),
        };

        children.retain(|parent_child| *parent_child != child);
        Ok(())
    }

    pub fn add_child(&mut self, parent: Ino, child: Ino) -> WhResult<()> {
        let parent = self.get_inode_mut(parent)?;

        let children = match &mut parent.entry {
            FsEntry::Directory(children) => Ok(children),
            _ => Err(WhError::InodeIsNotADirectory),
        }?;

        children.push(child);
        Ok(())
    }

    /// Remove inode from the [ITree]
    pub fn remove_inode(&mut self, id: Ino) -> Result<Inode, RemoveInodeError> {
        let inode = self.get_inode(id)?;
        match &inode.entry {
            FsEntry::Directory(children) if !children.is_empty() => {
                return Err(RemoveInodeError::NonEmpty)
            }
            _ => {}
        }

        self.remove_child(inode.parent, inode.id)?;

        self.entries.remove(&id).ok_or(RemoveInodeError::WhError {
            source: WhError::InodeNotFound,
        })
    }

    pub fn get_inode(&self, ino: Ino) -> WhResult<&Inode> {
        self.entries.get(&ino).ok_or(WhError::InodeNotFound)
    }

    pub fn mv_inode(
        &mut self,
        parent: Ino,
        new_parent: Ino,
        name: &str,
        new_name: InodeName,
    ) -> WhResult<()> {
        let parent_inode = self.entries.get(&parent).ok_or(WhError::InodeNotFound)?;
        let item_id = self.get_inode_child_by_name(parent_inode, name)?.id;

        self.remove_child(parent, item_id)?;

        let item = self.get_inode_mut(item_id)?;
        item.name = new_name;
        item.parent = new_parent;

        self.add_child(new_parent, item_id)
    }

    //REVIEW: This restriction seems execisve, it keep making me write unclear code and make the process tedious,
    //obligate us to create too many one liners while keeping the same "problem" of not propagating the change to the other inode
    //Performance is very important with this project so we should not force ourself to take a ass-backward way each time we interact with the itree
    ////REMOVED: not public as the modifications are not automaticly propagated on other related inodes
    pub fn get_inode_mut(&mut self, ino: Ino) -> WhResult<&mut Inode> {
        self.entries.get_mut(&ino).ok_or(WhError::InodeNotFound)
    }

    /// Recursively traverse the [ITree] tree from the [Inode] to form a path
    ///
    /// Possible Errors:
    ///   InodeNotFound: if the inode isn't inside the tree
    pub fn get_path_from_inode_id(&self, inode_index: Ino) -> WhResult<WhPath> {
        if inode_index == ROOT {
            return Ok(WhPath::root());
        }
        let inode = self
            .entries
            .get(&inode_index)
            .ok_or(WhError::InodeNotFound)?;

        let mut parent_path = self.get_path_from_inode_id(inode.parent)?;
        parent_path.push((&inode.name).into());
        Ok(parent_path)
    }

    pub fn get_inode_child_by_name(&self, parent: &Inode, name: &str) -> WhResult<&Inode> {
        if let Ok(children) = parent.entry.get_children() {
            for child in children.iter() {
                if let Some(child) = self.entries.get(child) {
                    if child.name == *name {
                        return Ok(child);
                    }
                }
            }
            Err(WhError::InodeNotFound)
        } else {
            Err(WhError::InodeIsNotADirectory)
        }
    }

    pub fn get_inode_from_path(&self, path: &WhPath) -> WhResult<&Inode> {
        let mut actual_inode = self.entries.get(&ROOT).expect("inode_from_path: NO ROOT");

        for name in path.iter() {
            actual_inode = self.get_inode_child_by_name(actual_inode, name)?;
        }

        Ok(actual_inode)
    }

    /// Get the hosts of a file
    pub fn get_inode_hosts(&self, ino: Ino) -> WhResult<&[PeerId]> {
        let inode = self.get_inode(ino)?;

        if let FsEntry::File(hosts) = &inode.entry {
            Ok(hosts)
        } else {
            Err(WhError::InodeIsADirectory)
        }
    }

    /// Add hosts to an inode
    ///
    /// Only works on inodes pointing files (no folders)
    /// Ignore already existing hosts to avoid duplicates
    pub fn add_inode_hosts(&mut self, ino: Ino, new_hosts: &[PeerId]) -> WhResult<()> {
        let inode = self.get_inode_mut(ino)?;

        if let FsEntry::File(hosts) = &mut inode.entry {
            hosts.extend_from_slice(new_hosts);
            hosts.sort();
            hosts.dedup();
            Ok(())
        } else {
            Err(WhError::InodeIsADirectory)
        }
    }

    /// Remove hosts from an inode
    ///
    /// Only works on inodes pointing files (no folders)
    pub fn remove_inode_hosts(&mut self, ino: Ino, remove_hosts: &[PeerId]) -> WhResult<()> {
        let inode = self.get_inode_mut(ino)?;

        match &mut inode.entry {
            FsEntry::File(old_hosts) => old_hosts.retain(|host| !remove_hosts.contains(host)),
            _ => return Err(WhError::InodeIsADirectory),
        };
        Ok(())
    }

    pub fn set_inode_meta(&mut self, ino: Ino, meta: Metadata) -> WhResult<()> {
        let inode = self.get_inode_mut(ino)?;

        inode.meta = meta;
        Ok(())
    }

    pub fn set_inode_size(&mut self, ino: Ino, size: u64) -> WhResult<()> {
        self.get_inode_mut(ino)?.meta.size = size;
        Ok(())
    }

    pub fn set_inode_xattr(&mut self, ino: Ino, key: &str, data: Vec<u8>) -> WhResult<()> {
        let inode = self.get_inode_mut(ino)?;

        inode.xattrs.insert(key.into(), data);
        Ok(())
    }

    pub fn remove_inode_xattr(&mut self, ino: Ino, key: &str) -> WhResult<()> {
        let inode = self.get_inode_mut(ino)?;

        inode.xattrs.remove(key);
        Ok(())
    }

    pub fn get_file_tree_and_hosts(&self, path: Option<&WhPath>) -> WhResult<Vec<TreeLine>> {
        let ino = if let Some(path) = path {
            self.get_inode_from_path(path)
                .map_err(|_| WhError::InodeNotFound)?
                .id
        } else {
            ROOT
        };

        self.recurse_tree(ino, 0)
    }

    /// given ino is not checked -> must exist in itree
    fn recurse_tree(&self, ino: Ino, indentation: u8) -> WhResult<Vec<TreeLine>> {
        let entry = &self.get_inode(ino)?.entry;
        let path = self.get_path_from_inode_id(ino)?;
        match entry {
            FsEntry::Directory(children) => Ok(children
                .iter()
                .map(|c| self.recurse_tree(*c, indentation + 1))
                .collect::<WhResult<Vec<Vec<TreeLine>>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<TreeLine>>()),
            entry => Ok(vec![(indentation, ino, path, entry.clone())]),
        }
    }
}

impl Default for ITree {
    fn default() -> ITree {
        ITree::new()
    }
}

// !SECTION

/// If itree can be read and deserialized from parent_folder/[ITREE_FILE_NAME] returns Some(ITree)
fn recover_serialized_itree(parent_folder: &Path) -> Option<ITree> {
    // error handling is silent on purpose as it will be recoded with the new error system
    // If an error happens, will just proceed like the itree was not on disk
    // In the future, we should maybe warn and keep a copy, avoiding the user from losing data
    bincode::deserialize(&fs::read(parent_folder.join(ITREE_FILE_FNAME)).ok()?).ok()
}

fn index_folder_recursive(
    itree: &mut ITree,
    parent: Ino,
    path: &Path,
    host: &PeerId,
    mountpoint: &Path,
) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry.expect("error in filesystem indexion (1)");
        let fname: InodeName = entry
            .file_name()
            .try_into()
            .map_err(|e: InodeNameError| e.to_io())?;
        let meta = entry.metadata()?;

        let ftype = meta.file_type();

        let special_ino = ITree::get_special(fname.as_ref(), parent);

        let used_ino = match special_ino {
            Some(_) if !ftype.is_file() => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Protected name is a folder",
                ))
            }
            Some(ino) => ino,
            None => itree
                .next_ino
                .next()
                .ok_or(io::Error::other("ran out of Inodes"))?,
        };

        #[cfg(target_os = "linux")]
        let perm_mode = meta.permissions().mode() as u16;
        #[cfg(target_os = "windows")]
        let perm_mode = WINDOWS_DEFAULT_PERMS_MODE;

        let fs_entry = match ftype.try_into()? {
            SimpleFileType::Directory => FsEntry::new_directory(),
            SimpleFileType::File => FsEntry::File(vec![*host]),
            SimpleFileType::Symlink => {
                let target = std::fs::read_link(entry.path());
                let link = EntrySymlink::parse(&target?, mountpoint);
                FsEntry::Symlink(link.unwrap_or_else(|e| e))
            }
        };

        itree
            .add_inode(Inode::new(
                fname.clone(),
                parent,
                used_ino,
                fs_entry,
                perm_mode,
            ))
            .map_err(io::Error::other)?;
        let mut meta: Metadata = meta.try_into()?;
        meta.ino = used_ino;
        itree
            .set_inode_meta(used_ino, meta)
            .map_err(io::Error::other)?;

        if ftype.is_dir() {
            index_folder_recursive(itree, used_ino, &path.join(&fname), host, mountpoint)
                .expect("error in filesystem indexion (3)");
        };
    }
    Ok(())
}
