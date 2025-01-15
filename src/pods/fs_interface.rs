use crate::network::message::Address;

use super::whpath::WhPath;
use super::{
    arbo::{Arbo, FsEntry, Inode, InodeId, LOCK_TIMEOUT},
    disk_manager::DiskManager,
    network_interface::NetworkInterface,
};
use parking_lot::RwLock;
use std::io::{self};
use std::sync::Arc;

pub struct FsInterface {
    pub network_interface: Arc<NetworkInterface>,
    pub disk: DiskManager,
    pub arbo: Arc<RwLock<Arbo>>, // here only to read, as most write are made by network_interface
                                 // REVIEW - check self.arbo usage to be only reading
}

pub enum SimpleFileType {
    File,
    Directory,
}

/// Provides functions to allow primitive handlers like Fuse & WinFSP to
/// interract with wormhole.
impl FsInterface {
    pub fn new(
        network_interface: Arc<NetworkInterface>,
        disk_manager: DiskManager,
        arbo: Arc<RwLock<Arbo>>,
    ) -> Self {
        Self {
            network_interface,
            disk: disk_manager,
            arbo,
        }
    }

    // SECTION - local -> write

    pub fn make_inode(
        &self,
        parent_ino: u64,
        name: String,
        kind: SimpleFileType,
    ) -> io::Result<(InodeId, Inode)> {
        let new_entry = match kind {
            SimpleFileType::File => FsEntry::File(Vec::new()),
            SimpleFileType::Directory => FsEntry::Directory(Vec::new()),
        };

        let new_inode_id = self.network_interface.get_next_inode()?;
        let new_inode: Inode = Inode::new(name, parent_ino, new_inode_id, new_entry);
        self.network_interface
            .register_new_file(new_inode.clone())?;

        let new_path: WhPath = if let Some(arbo) = self.arbo.try_read_for(LOCK_TIMEOUT) {
            arbo.get_path_from_inode_id(new_inode_id)?
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "mkfile: can't read lock arbo's RwLock",
            ));
        };

        match self.disk.new_file(&new_path) {
            Ok(_) => (),
            Err(e) => {
                return Err(e);
            }
        };

        Ok((new_inode_id, new_inode))
    }

    pub fn remove_inode(&self, id: InodeId) -> io::Result<()> {
        let (to_remove_path, entry) = {
            let arbo = Arbo::read_lock(&self.arbo, "fs_interface::remove_inode")?;
            (
                arbo.get_path_from_inode_id(id)?,
                arbo.get_inode(id)?.entry.clone(),
            )
        };

        match entry {
            FsEntry::File(_) => self.disk.remove_file(&to_remove_path),
            FsEntry::Directory(children) => {
                if children.is_empty() {
                    self.disk.remove_dir(&to_remove_path)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "remove_inode: can't remove non-empty dir",
                    ))
                }
            }
        }?;

        self.network_interface.unregister_file(id)?;

        Ok(())
    }

    pub fn write(&self, id: InodeId, data: Vec<u8>, offset: u64) -> io::Result<u64> {
        let written = {
            let arbo = Arbo::read_lock(&self.arbo, "fs_interface.write")?;
            self.disk
                .write_file(&arbo.get_path_from_inode_id(id)?, data, offset)?
        };

        self.network_interface.revoke_remote_hosts(id)?; // TODO - manage this error to prevent remote/local desync
        Ok(written)
    }
    // !SECTION

    // SECTION - local -> read

    pub fn get_entry_from_name(&self, parent: InodeId, name: String) -> io::Result<Inode> {
        let arbo = Arbo::read_lock(&self.arbo, "fs_interface.get_entry_from_name")?;
        arbo.get_inode_child_by_name(arbo.get_inode(parent)?, &name)
            .cloned()
    }

    pub fn read_file(&self, file: InodeId, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        let status = self
            .network_interface
            .pull_file(file)?
            .blocking_recv() // NOTE - blocking_recv doc does not indicate in what case None is returned
            .expect("read_file: blocking_recev returned None");

        if status == false {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "file already waiting for pull or unable to write pulled to disk",
            ));
        }

        self.disk.read_file(
            &Arbo::read_lock(&self.arbo, "read_file")?.get_path_from_inode_id(file)?,
            offset,
            len,
        )
    }

    pub fn read_dir(&self, ino: InodeId) -> io::Result<Vec<Inode>> {
        let arbo = Arbo::read_lock(&self.arbo, "fs_interface.read_dir")?;
        let dir = arbo.get_inode(ino)?;
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
    pub fn recept_inode(&self, inode: Inode, id: InodeId) -> io::Result<()> {
        self.network_interface.acknowledge_new_file(inode, id)?;

        let new_path = {
            let arbo = Arbo::read_lock(&self.arbo, "fs_interface.write")?;
            arbo.get_path_from_inode_id(id)?
        };

        match self.disk.new_file(&new_path) {
            Ok(_) => (),
            Err(e) => {
                return Err(e);
            }
        };

        Ok(())
    }

    pub fn recept_binary(&self, id: InodeId, binary: Vec<u8>) {
        let path = {
            let arbo = Arbo::read_lock(&self.arbo, "recept_binary")
                .expect("recept_binary: can't read lock arbo");

            match arbo.get_path_from_inode_id(id) {
                Ok(path) => path,
                Err(_) => return self.network_interface.resolve_pull(id, false),
            }
        };
        let status = self.disk.write_file(&path, binary, 0).is_ok();
        self.network_interface.resolve_pull(id, status);
    }

    pub fn recept_remove_inode(&self, id: InodeId) -> io::Result<()> {
        let to_remove_path = {
            let arbo = Arbo::read_lock(&self.arbo, "cecept_remove_inode")?;
            arbo.get_path_from_inode_id(id)?
        };

        // REVIEW - should be ok that file is not on disk
        match self.disk.remove_file(&to_remove_path) {
            Ok(_) => (),
            Err(e) => {
                return Err(e);
            }
        };

        self.network_interface.acknowledge_unregister_file(id)?;

        Ok(())
    }

    pub fn recept_edit_hosts(&self, id: InodeId, hosts: Vec<Address>) {
        self.network_interface.acknowledge_hosts_edition(id, hosts);
    }
    // !SECTION

    // SECTION - Adapters
    // NOTE - system specific (fuse/winfsp) code that need access to arbo or other classes

    pub fn fuse_remove_inode(&self, parent: InodeId, name: &std::ffi::OsStr) -> io::Result<()> {
        let target = {
            let arbo = Arbo::read_lock(&self.arbo, "fs_interface::fuse_remove_inode")?;
            let parent = arbo.get_inode(parent)?;
            arbo.get_inode_child_by_name(parent, &name.to_string_lossy().to_string())?
                .id
        };

        self.remove_inode(target)
    }

    // !SECTION
}
