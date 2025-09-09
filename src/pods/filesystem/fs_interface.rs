use crate::error::WhResult;
use crate::network::message::Address;
use crate::pods::arbo::{Arbo, FsEntry, Inode, InodeId, Metadata, GLOBAL_CONFIG_INO};
use crate::pods::disk_managers::DiskManager;
use crate::pods::filesystem::attrs::AcknoledgeSetAttrError;
use crate::pods::network::callbacks::Callback;
use crate::pods::network::network_interface::NetworkInterface;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};
use std::sync::Arc;

use super::file_handle::FileHandleManager;
use super::make_inode::MakeInodeError;

#[derive(Debug)]
pub struct FsInterface {
    pub network_interface: Arc<NetworkInterface>,
    pub disk: Box<dyn DiskManager>,
    pub file_handles: Arc<RwLock<FileHandleManager>>,
    pub arbo: Arc<RwLock<Arbo>>, // here only to read, as most write are made by network_interface
                                 // REVIEW - check self.arbo usage to be only reading
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub enum SimpleFileType {
    File,
    Directory,
}

impl Into<SimpleFileType> for &FsEntry {
    fn into(self) -> SimpleFileType {
        match self {
            FsEntry::File(_) => SimpleFileType::File,
            FsEntry::Directory(_) => SimpleFileType::Directory,
        }
    }
}

/// Provides functions to allow primitive handlers like Fuse & WinFSP to
/// interract with wormhole
impl FsInterface {
    pub fn new(
        network_interface: Arc<NetworkInterface>,
        disk_manager: Box<dyn DiskManager>,
        arbo: Arc<RwLock<Arbo>>,
    ) -> Self {
        Self {
            network_interface,
            disk: disk_manager,
            file_handles: Arc::new(RwLock::new(FileHandleManager::new())),
            arbo,
        }
    }

    // SECTION - local -> write
    #[deprecated]
    pub fn set_inode_meta(&self, ino: InodeId, meta: Metadata) -> io::Result<()> {
        let path = Arbo::read_lock(&self.arbo, "fs_interface::set_inode_meta")?
            .get_path_from_inode_id(ino)?;

        self.disk.set_file_size(&path, meta.size as usize)?;
        self.network_interface
            .update_metadata(ino, meta)
            .map_err(|err| std::io::Error::new(ErrorKind::Other, err))
    }

    // !SECTION

    // SECTION - local -> read

    pub fn get_entry_from_name(&self, parent: InodeId, name: String) -> io::Result<Inode> {
        let arbo = Arbo::read_lock(&self.arbo, "fs_interface.get_entry_from_name")?;
        arbo.get_inode_child_by_name(arbo.get_inode(parent)?, &name)
            .cloned()
    }

    pub fn get_inode_attributes(&self, ino: InodeId) -> io::Result<Metadata> {
        let arbo = Arbo::read_lock(&self.arbo, "fs_interface::get_inode_attributes")?;

        Ok(arbo.get_inode(ino)?.meta.clone())
    }

    pub fn n_get_inode_attributes(&self, ino: InodeId) -> WhResult<Metadata> {
        let arbo = Arbo::n_read_lock(&self.arbo, "fs_interface::get_inode_attributes")?;

        Ok(arbo.n_get_inode(ino)?.meta.clone())
    }

    pub fn read_dir(&self, ino: InodeId) -> io::Result<Vec<Inode>> {
        let arbo = Arbo::read_lock(&self.arbo, "fs_interface.read_dir")?;
        let dir = arbo.get_inode(ino)?;
        //log::debug!("dir: {dir}?");
        let mut entries: Vec<Inode> = Vec::new();

        if let FsEntry::Directory(children) = &dir.entry {
            for entry in children {
                entries.push(arbo.get_inode(*entry)?.clone());
            }
            Ok(entries)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "read_dir: asked inode is not a dir",
            ))
        }
    }
    // !SECTION

    // SECTION - remote -> write
    pub fn recept_inode(&self, inode: Inode) -> Result<(), MakeInodeError> {
        self.network_interface
            .acknowledge_new_file(inode.clone(), inode.id)?;
        self.network_interface.promote_next_inode(inode.id + 1)?;

        let new_path = {
            let arbo = Arbo::n_read_lock(&self.arbo, "recept_inode")?;
            arbo.n_get_path_from_inode_id(inode.id)?
        };

        match inode.entry {
            // REVIEW - is it still useful to create an empty file in this case ?
            FsEntry::File(hosts) if hosts.contains(&self.network_interface.hostname()?) => self
                .disk
                .new_file(&new_path, inode.meta.perm)
                .map(|_| ())
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
            FsEntry::File(_) => Ok(()),
            FsEntry::Directory(_) => self
                .disk
                .new_dir(&new_path, inode.meta.perm)
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
            // TODO - remove when merge is handled because new file should create folder
            // FsEntry::Directory(_) => {}
        }
    }

    pub fn recept_redundancy(&self, id: InodeId, binary: Arc<Vec<u8>>) -> WhResult<()> {
        let arbo = Arbo::write_lock(&self.arbo, "recept_binary")
            .expect("recept_binary: can't read lock arbo");
        let (path, perms) = arbo
            .n_get_path_from_inode_id(id)
            .and_then(|path| arbo.n_get_inode(id).map(|inode| (path, inode.meta.perm)))?;
        drop(arbo);

        let _created = self.disk.new_file(&path, perms);
        self.disk
            .write_file(&path, &binary, 0)
            .inspect_err(|e| log::error!("{e}"))
            .expect("disk error");
        // TODO -> in case of failure, other hosts still think this one is valid. Should send error report to the redundancy manager

        Arbo::n_write_lock(&self.arbo, "recept_redundancy")?
            .n_add_inode_hosts(id, vec![self.network_interface.hostname()?])
            .inspect_err(|e| {
                log::error!("Can't update (local) hosts for redundancy pulled file ({id}): {e}")
            })
    }

    pub fn recept_binary(&self, id: InodeId, binary: Vec<u8>) -> io::Result<()> {
        let arbo = Arbo::read_lock(&self.arbo, "recept_binary")
            .expect("recept_binary: can't read lock arbo");
        let (path, perms) = match arbo
            .n_get_path_from_inode_id(id)
            .and_then(|path| arbo.n_get_inode(id).map(|inode| (path, inode.meta.perm)))
        {
            Ok(value) => value,
            Err(_) => {
                return self
                    .network_interface
                    .callbacks
                    .resolve(Callback::Pull(id), false)
            }
        };
        drop(arbo);
        let _created = self.disk.new_file(&path, perms);
        let status = self
            .disk
            .write_file(&path, &binary, 0)
            .inspect_err(|e| log::error!("writing pulled file: {e}"));
        let _ = self
            .network_interface
            .callbacks
            .resolve(Callback::Pull(id), status.is_ok());
        status?;
        self.network_interface
            .add_inode_hosts(
                id,
                vec![self
                    .network_interface
                    .hostname()
                    .ok()
                    .ok_or(io::Error::other("deadlock"))?],
            )
            .expect("can't update inode hosts");
        Ok(())
    }

    pub fn recept_edit_hosts(&self, id: InodeId, hosts: Vec<Address>) -> WhResult<()> {
        if !hosts.contains(&self.network_interface.hostname()?) {
            let path =
                Arbo::n_read_lock(&self.arbo, "recept_edit_hosts")?.n_get_path_from_inode_id(id)?;
            if let Err(e) = self.disk.remove_file(&path) {
                log::debug!("recept_edit_hosts: can't delete file. {}", e);
            }
        }
        self.network_interface.acknowledge_hosts_edition(id, hosts)
    }

    pub fn recept_revoke_hosts(
        &self,
        id: InodeId,
        host: String,
        meta: Metadata,
    ) -> Result<(), AcknoledgeSetAttrError> {
        let needs_delete = host != self.network_interface.hostname()?;
        self.acknowledge_metadata(id, meta)?;
        self.network_interface
            .acknowledge_hosts_edition(id, vec![host])
            .map_err(|source| AcknoledgeSetAttrError::WhError { source })?;
        if needs_delete {
            // TODO: recept_revoke_hosts, for the redudancy, should recieve the written text (data from write) instead of deleting and adding it back completely with apply_redudancy
            if let Err(e) = self.disk.remove_file(
                &Arbo::n_read_lock(&self.arbo, "recept_revoke_hosts")?
                    .n_get_path_from_inode_id(id)?,
            ) {
                log::debug!("recept_revoke_hosts: can't delete file. {}", e);
            }
        }
        Ok(())
    }

    pub fn recept_add_hosts(&self, id: InodeId, hosts: Vec<Address>) -> io::Result<()> {
        self.network_interface.aknowledge_new_hosts(id, hosts)
    }

    pub fn recept_remove_hosts(&self, id: InodeId, hosts: Vec<Address>) -> io::Result<()> {
        if hosts.contains(
            &self
                .network_interface
                .hostname()
                .ok()
                .ok_or(io::Error::other("deadlock"))?,
        ) {
            if let Err(e) = self.disk.remove_file(
                &Arbo::read_lock(&self.arbo, "recept_remove_hosts")?.get_path_from_inode_id(id)?,
            ) {
                log::debug!("recept_remove_hosts: can't delete file. {}", e);
            }
        }

        self.network_interface.aknowledge_hosts_removal(id, hosts)
    }

    // !SECTION

    // SECTION remote -> read
    pub fn send_filesystem(&self, to: Address) -> io::Result<()> {
        let arbo = Arbo::read_lock(&self.arbo, "fs_interface::send_filesystem")?;
        let global_config_file_size = arbo
            .get_inode(GLOBAL_CONFIG_INO)
            .map(|inode| inode.meta.size)
            .ok();
        let global_config_path = if global_config_file_size.is_some() {
            Some(
                arbo.get_path_from_inode_id(GLOBAL_CONFIG_INO)?
                    .set_relative(),
            )
        } else {
            None
        };
        drop(arbo);
        log::info!("reading global config at {global_config_path:?}");

        let mut global_config_bytes = Vec::new();
        if let Some(global_config_file_size) = global_config_file_size {
            global_config_bytes.resize(global_config_file_size as usize, 0);

            if let Some(global_config_path) = global_config_path {
                self.disk
                    .read_file(&global_config_path, 0, &mut global_config_bytes)
                    .expect("disk can't read file (global condfig)");
            }
        }
        self.network_interface.send_arbo(to, global_config_bytes)
    }

    pub fn send_file(&self, inode: InodeId, to: Address) -> io::Result<()> {
        let arbo = Arbo::read_lock(&self.arbo, "send_arbo")?;
        let path = arbo.get_path_from_inode_id(inode)?;
        let mut size = arbo.get_inode(inode)?.meta.size as usize;
        let mut data = Vec::new();
        data.resize(size, 0);
        size = self.disk.read_file(&path, 0, &mut data)?;
        data.resize(size, 0);
        self.network_interface.send_file(inode, data, to)
    }

    pub fn read_local_file(&self, inode: InodeId) -> WhResult<Vec<u8>> {
        let arbo = Arbo::n_read_lock(&self.arbo, "send_arbo")?;
        let path = arbo
            .get_path_from_inode_id(inode)
            .map_err(|_| crate::error::WhError::InodeNotFound)?;
        let size = arbo.n_get_inode(inode)?.meta.size;
        drop(arbo);

        let mut buff = Vec::new();
        buff.resize(size as usize, 0);
        self.disk
            .read_file(&path, 0, &mut buff)
            .map_err(|_| crate::error::WhError::InodeNotFound)?;
        Ok(buff)
    }
}
