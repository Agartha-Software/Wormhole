use crate::error::WhResult;
use crate::network::message::Response;
use crate::pods::disk_managers::DiskManager;
use crate::pods::filesystem::attrs::AcknoledgeSetAttrError;
use crate::pods::filesystem::permissions::has_execute_perm;
use crate::pods::itree::{FsEntry, ITree, Ino, Inode, Metadata};
use crate::pods::network::network_interface::NetworkInterface;

use futures::io;
use libp2p::PeerId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use super::file_handle::FileHandleManager;
use super::make_inode::MakeInodeError;

pub struct FsInterface {
    pub network_interface: Arc<NetworkInterface>,
    pub disk: Box<dyn DiskManager>,
    pub file_handles: Arc<RwLock<FileHandleManager>>,
    pub mountpoint: PathBuf,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum SimpleFileType {
    File,
    Directory,
    Symlink,
}

impl From<&FsEntry> for SimpleFileType {
    fn from(entry: &FsEntry) -> SimpleFileType {
        match entry {
            FsEntry::File(_) => SimpleFileType::File,
            FsEntry::Directory(_) => SimpleFileType::Directory,
            FsEntry::Symlink(_) => SimpleFileType::Symlink,
        }
    }
}

impl TryFrom<std::fs::FileType> for SimpleFileType {
    type Error = std::io::Error;
    fn try_from(entry: std::fs::FileType) -> Result<Self, Self::Error> {
        match (entry.is_file(), entry.is_dir(), entry.is_symlink()) {
            (true, false, false) => Ok(SimpleFileType::File),
            (false, true, false) => Ok(SimpleFileType::Directory),
            (false, false, true) => Ok(SimpleFileType::Symlink),
            _ => Err(io::ErrorKind::PermissionDenied.into()),
        }
    }
}

/// Provides functions to allow primitive handlers like Fuse & WinFSP to
/// interract with wormhole
impl FsInterface {
    pub fn new(
        network_interface: Arc<NetworkInterface>,
        disk_manager: Box<dyn DiskManager>,
        mountpoint: PathBuf,
    ) -> Self {
        Self {
            network_interface,
            disk: disk_manager,
            file_handles: Arc::new(RwLock::new(FileHandleManager::new())),
            mountpoint,
        }
    }

    // SECTION - local -> read

    /// get an entry
    /// return Ok(None) if no permissions to access entries
    pub fn get_entry_from_name(&self, parent: Ino, name: &str) -> WhResult<Option<Inode>> {
        let itree = ITree::read_lock(
            &self.network_interface.itree,
            "fs_interface.get_entry_from_name",
        )?;
        let p_inode = itree.get_inode(parent)?;
        if !has_execute_perm(p_inode.meta.perm) {
            return Ok(None);
        }
        Ok(Some(itree.get_inode_child_by_name(p_inode, name)?.clone()))
    }

    pub fn get_inode_attributes(&self, ino: Ino) -> WhResult<Metadata> {
        let itree = ITree::read_lock(
            &self.network_interface.itree,
            "fs_interface::get_inode_attributes",
        )?;

        Ok(itree.get_inode(ino)?.meta.clone())
    }

    // !SECTION

    // SECTION - remote -> write
    pub fn recept_inode(&self, inode: Inode) -> Result<Response, MakeInodeError> {
        self.network_interface.acknowledge_new_file(inode.clone())?;

        let new_path = {
            let itree = ITree::read_lock(&self.network_interface.itree, "recept_inode")?;
            itree.get_path_from_inode_id(inode.id)?
        };

        match &inode.entry {
            // REVIEW - is it still useful to create an empty file in this case ?
            FsEntry::File(hosts) if hosts.contains(&self.network_interface.id) => self
                .disk
                .new_file(&new_path, inode.meta.perm)
                .map(|_| ())
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
            FsEntry::File(_) => Ok(()),
            FsEntry::Directory(_) => self
                .disk
                .new_dir(&new_path, inode.meta.perm)
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
            FsEntry::Symlink(symlink) => self
                .disk
                .new_symlink(&new_path, inode.meta.perm, symlink)
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
            // TODO - remove when merge is handled because new file should create folder
            // FsEntry::Directory(_) => {}
        }?;
        Ok(Response::Success)
    }

    pub fn recept_redundancy(&self, id: Ino, binary: Arc<Vec<u8>>) -> WhResult<Response> {
        let itree = ITree::write_lock(&self.network_interface.itree, "recept_binary")
            .expect("recept_binary: can't read lock itree");
        let (path, perms) = itree
            .get_path_from_inode_id(id)
            .and_then(|path| itree.get_inode(id).map(|inode| (path, inode.meta.perm)))?;
        drop(itree);

        let _created = self.disk.new_file(&path, perms);
        self.disk
            .write_file(&path, &binary, 0)
            .inspect_err(|e| log::error!("{e}"))
            .expect("disk error");
        // TODO -> in case of failure, other hosts still think this one is valid. Should send error report to the redundancy manager

        ITree::write_lock(&self.network_interface.itree, "recept_redundancy")?
            .add_inode_hosts(id, vec![self.network_interface.id.clone()])
            .inspect_err(|e| {
                log::error!("Can't update (local) hosts for redundancy pulled file ({id}): {e}")
            })?;
        Ok(Response::Success)
    }

    pub fn recept_edit_hosts(&self, id: Ino, hosts: Vec<PeerId>) -> WhResult<Response> {
        if !hosts.contains(&self.network_interface.id) {
            let path = ITree::read_lock(&self.network_interface.itree, "recept_edit_hosts")?
                .get_path_from_inode_id(id)?;
            if let Err(e) = self.disk.remove_file(&path) {
                log::debug!("recept_edit_hosts: can't delete file. {}", e);
            }
        }
        self.network_interface
            .acknowledge_hosts_edition(id, hosts)?;
        Ok(Response::Success)
    }

    pub fn recept_revoke_hosts(
        &self,
        id: Ino,
        host: PeerId,
        meta: Metadata,
    ) -> Result<Response, AcknoledgeSetAttrError> {
        let needs_delete = host != self.network_interface.id;
        self.acknowledge_metadata(id, meta)?;
        self.network_interface
            .acknowledge_hosts_edition(id, vec![host])
            .map_err(|source| AcknoledgeSetAttrError::WhError { source })?;
        if needs_delete {
            // TODO: recept_revoke_hosts, for the redudancy, should recieve the written text (data from write) instead of deleting and adding it back completely with apply_redudancy
            if let Err(e) = self.disk.remove_file(
                &ITree::read_lock(&self.network_interface.itree, "recept_revoke_hosts")?
                    .get_path_from_inode_id(id)?,
            ) {
                log::debug!("recept_revoke_hosts: can't delete file. {}", e);
            }
        }
        Ok(Response::Success)
    }

    pub fn recept_add_hosts(&self, id: Ino, hosts: Vec<PeerId>) -> WhResult<Response> {
        self.network_interface.aknowledge_new_hosts(id, hosts)?;
        Ok(Response::Success)
    }

    pub fn recept_remove_hosts(&self, id: Ino, hosts: Vec<PeerId>) -> WhResult<Response> {
        if hosts.contains(&self.network_interface.id) {
            if let Err(e) = self.disk.remove_file(
                &ITree::read_lock(&self.network_interface.itree, "recept_remove_hosts")?
                    .get_path_from_inode_id(id)?,
            ) {
                log::debug!("recept_remove_hosts: can't delete file. {}", e);
            }
        }

        self.network_interface.aknowledge_hosts_removal(id, hosts)?;
        Ok(Response::Success)
    }

    // !SECTION

    // SECTION remote -> read
    pub fn send_file(&self, inode: Ino) -> io::Result<Response> {
        let itree = ITree::read_lock(&self.network_interface.itree, "send_itree")
            .map_err(io::Error::other)?;
        let path = itree
            .get_path_from_inode_id(inode)
            .map_err(io::Error::other)?;
        let mut size = itree.get_inode(inode).map_err(io::Error::other)?.meta.size as usize;
        let mut data = vec![0; size];
        size = self.disk.read_file(&path, 0, &mut data)?;
        data.resize(size, 0);
        Ok(Response::RequestedFile(data))
    }

    pub fn read_local_file(&self, inode: Ino) -> WhResult<Vec<u8>> {
        let itree = ITree::read_lock(&self.network_interface.itree, "send_itree")?;
        let path = itree
            .get_path_from_inode_id(inode)
            .map_err(|_| crate::error::WhError::InodeNotFound)?;
        let size = itree.get_inode(inode)?.meta.size;
        drop(itree);

        let mut buff = vec![0; size as usize];
        self.disk
            .read_file(&path, 0, &mut buff)
            .map_err(|_| crate::error::WhError::InodeNotFound)?;
        Ok(buff)
    }

    //REVIEW - I don't really like to lock the arbo here, but it's the only way to get the inode countwithout just using an arbitrary high number
    /// Get complete filesystem size information including inode counts
    pub fn get_size_info(&self) -> io::Result<crate::pods::disk_managers::DiskSizeInfo> {
        let mut disk_info = self.disk.size_info()?;

        let itree = ITree::read_lock(&self.network_interface.itree, "fs_interface::get_size_info")
            .map_err(|_| {
                io::Error::other(crate::error::WhError::WouldBlock {
                    called_from: "fs_interface::get_size_info".to_string(),
                })
            })?;
        let files = itree.iter().count() as u64;
        let next_ino = itree.next_ino.start;

        let ffree = if next_ino < u64::MAX / 2 {
            u64::MAX - next_ino
        } else {
            1_000_000_000
        };

        disk_info.files = files;
        disk_info.ffree = ffree;

        Ok(disk_info)
    }
}
