use crate::config::{types::Config, LocalConfig};
use crate::error::{WhError, WhResult};
use crate::network::message::Address;
use crate::pods::arbo::{Arbo, FsEntry, Inode, InodeId, Metadata, GLOBAL_CONFIG_INO};
use crate::pods::disk_manager::DiskManager;
use crate::pods::network::callbacks::Callback;
use crate::pods::network::network_interface::NetworkInterface;
use crate::pods::whpath::WhPath;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};
use std::sync::Arc;

use super::file_handle::FileHandleManager;
use super::make_inode::MakeInodeError;

#[derive(Debug)]
pub struct FsInterface {
    pub network_interface: Arc<NetworkInterface>,
    pub disk: DiskManager,
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
        disk_manager: DiskManager,
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
    pub fn set_inode_meta(&self, ino: InodeId, meta: Metadata) -> io::Result<()> {
        let path = Arbo::read_lock(&self.arbo, "fs_interface::set_inode_meta")?
            .get_path_from_inode_id(ino)?;

        self.disk.set_file_size(path, meta.size)?;
        self.network_interface.update_metadata(ino, meta)
    }

    fn construct_file_path(&self, parent: InodeId, name: &String) -> io::Result<WhPath> {
        let arbo = Arbo::read_lock(&self.arbo, "fs_interface.get_begin_path_end_path")?;
        let parent_path = arbo.get_path_from_inode_id(parent)?;

        return Ok(parent_path.join(name));
    }

    // TODO:  Must handle the file creation if the file is not replicated like ino 3
    pub fn rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: &String,
        new_name: &String,
    ) -> io::Result<()> {
        let parent_path = self.construct_file_path(parent, name)?;
        let new_parent_path = self.construct_file_path(new_parent, new_name)?;
        let err = self
            .disk
            .mv_file(parent_path, new_parent_path)
            .inspect_err(|err| log::error!("disk.mv_file fail: {err:?}"));
        self.network_interface
            .broadcast_rename_file(parent, new_parent, name, new_name)?;
        self.network_interface
            .arbo_rename_file(parent, new_parent, name, new_name)?;
        Ok(())
    }

    pub fn accept_rename(
        &self,
        parent: InodeId,
        new_parent: InodeId,
        name: &String,
        new_name: &String,
    ) -> io::Result<()> {
        let parent_path = self.construct_file_path(parent, name)?;
        let new_parent_path = self.construct_file_path(new_parent, new_name)?;
        if std::path::Path::new(&parent_path.inner).exists() {
            self.disk.mv_file(parent_path, new_parent_path)?;
        }
        self.network_interface
            .arbo_rename_file(parent, new_parent, name, new_name)?;
        Ok(())
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
            FsEntry::File(hosts)
                if hosts.contains(
                    &LocalConfig::read_lock(&self.network_interface.local_config, "recept_inode")?
                        .general
                        .address,
                ) =>
            {
                self.disk
                    .new_file(new_path, inode.meta.perm)
                    .map(|_| ())
                    .map_err(|io| MakeInodeError::LocalCreationFailed { io })
            }
            FsEntry::File(_) => Ok(()),
            FsEntry::Directory(_) => self
                .disk
                .new_dir(new_path, inode.meta.perm)
                .map(|_| ())
                .map_err(|io| MakeInodeError::LocalCreationFailed { io }),
            // TODO - remove when merge is handled because new file should create folder
            // FsEntry::Directory(_) => {}
        }
    }

    pub fn recept_redundancy(&self, id: InodeId, binary: Vec<u8>) -> WhResult<()> {
        let path = Arbo::read_lock(&self.arbo, "recept_binary")
            .expect("recept_binary: can't read lock arbo")
            .n_get_path_from_inode_id(id)?;

        self.disk
            .write_file(path, &binary, 0)
            .map_err(|e| WhError::DiskError {
                detail: format!("recept_redundancy: can't write file ({id})"),
                err: e,
            })
            .inspect_err(|e| log::error!("{e}"))?;
        // TODO -> in case of failure, other hosts still think this one is valid. Should send error report to the redundancy manager

        let address =
            LocalConfig::read_lock(&self.network_interface.local_config, "revoke_remote_hosts")?
                .general
                .address
                .clone();
        Arbo::n_write_lock(&self.arbo, "recept_redundancy")?
            .n_add_inode_hosts(id, vec![address])
            .inspect_err(|e| {
                log::error!("Can't update (local) hosts for redundancy pulled file ({id}): {e}")
            })
    }

    pub fn recept_binary(&self, id: InodeId, binary: Vec<u8>) -> io::Result<()> {
        let path = match Arbo::read_lock(&self.arbo, "recept_binary")
            .expect("recept_binary: can't read lock arbo")
            .n_get_path_from_inode_id(id)
        {
            Ok(path) => path,
            Err(_) => {
                return self
                    .network_interface
                    .callbacks
                    .resolve(Callback::Pull(id), false)
            }
        };

        self.disk.write_file(path, &binary, 0)?;
        let address =
            LocalConfig::read_lock(&self.network_interface.local_config, "revoke_remote_hosts")
                .map_err(|e| {
                    log::error!("recept_binary: can't get self address: {e}");
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("recept_binary: can't get self address: {e}"),
                    )
                })?
                .general
                .address
                .clone();

        let _ = self
            .network_interface
            .add_inode_hosts(id, vec![address])
            .inspect_err(|e| log::error!("Can't update hosts for pulled file ({id}): {e}"));
        let _ = self
            .network_interface
            .callbacks
            .resolve(Callback::Pull(id), true);
        Ok(())
    }

    pub fn recept_edit_hosts(&self, id: InodeId, hosts: Vec<Address>) -> WhResult<()> {
        if !hosts.contains(
            &LocalConfig::read_lock(&self.network_interface.local_config, "recept_binary")?
                .general
                .address,
        ) {
            if let Err(e) = self.disk.remove_file(
                Arbo::n_read_lock(&self.arbo, "recept_edit_hosts")?.n_get_path_from_inode_id(id)?,
            ) {
                log::debug!("recept_edit_hosts: can't delete file. {}", e);
            }
        }
        self.network_interface.acknowledge_hosts_edition(id, hosts)
    }

    pub fn recept_revoke_hosts(&self, id: InodeId, host: Address, meta: Metadata) -> WhResult<()> {
        if host
            != LocalConfig::read_lock(&self.network_interface.local_config, "recept_binary")?
                .general
                .address
        {
            // TODO: recept_revoke_hosts, for the redudancy, should recieve the written text (data from write) instead of deleting and adding it back completely with apply_redudancy
            if let Err(e) = self.disk.remove_file(
                Arbo::n_read_lock(&self.arbo, "recept_revoke_hosts")?
                    .n_get_path_from_inode_id(id)?,
            ) {
                log::debug!("recept_revoke_hosts: can't delete file. {}", e);
            }
        }
        self.network_interface.acknowledge_metadata(id, meta)?;
        self.network_interface
            .acknowledge_hosts_edition(id, vec![host])
    }

    pub fn recept_add_hosts(&self, id: InodeId, hosts: Vec<Address>) -> io::Result<()> {
        self.network_interface.aknowledge_new_hosts(id, hosts)
    }

    pub fn recept_remove_hosts(&self, id: InodeId, hosts: Vec<Address>) -> io::Result<()> {
        if hosts.contains(
            &LocalConfig::read_lock(&self.network_interface.local_config, "recept_remove_hosts")
                .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?
                .general
                .address,
        ) {
            if let Err(e) = self.disk.remove_file(
                Arbo::read_lock(&self.arbo, "recept_remove_hosts")?.get_path_from_inode_id(id)?,
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
        let global_config_file_size = arbo.get_inode(GLOBAL_CONFIG_INO)?.meta.size;
        let global_config_path = arbo
            .get_path_from_inode_id(GLOBAL_CONFIG_INO)?
            .set_relative();
        drop(arbo);
        log::info!("reading global config at {global_config_path}");
        let global_config_bytes = self
            .disk
            .read_file(global_config_path, 0, global_config_file_size)
            .expect("lmao l'incompétence");

        self.network_interface.send_arbo(to, global_config_bytes)
    }

    pub fn register_new_node(&self, socket: Address, addr: Address) {
        self.network_interface.register_new_node(socket, addr);
    }

    pub fn send_file(&self, inode: InodeId, to: Address) -> io::Result<()> {
        let path = Arbo::read_lock(&self.arbo, "send_arbo")?.get_path_from_inode_id(inode)?;
        let data = self.disk.read_file(path, 0, u64::max_value())?;
        self.network_interface.send_file(inode, data, to)
    }

    pub fn read_local_file(&self, inode: InodeId) -> WhResult<Vec<u8>> {
        let path = Arbo::n_read_lock(&self.arbo, "send_arbo")?
            .get_path_from_inode_id(inode)
            .map_err(|_| crate::error::WhError::InodeNotFound)?;
        self.disk
            .read_file(path, 0, u64::max_value())
            .map_err(|_| crate::error::WhError::InodeNotFound)
        // self.network_interface
        //     .send_file_redundancy(inode, data, to)
        //     .await
    }
}
