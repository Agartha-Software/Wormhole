use std::sync::Arc;

use crate::pods::filesystem::file_handle::{AccessMode, FileHandle, FileHandleManager, UUID};
use crate::pods::filesystem::File;
use crate::pods::itree::{FsEntry, ITree, Ino};
use crate::pods::network::pull_file::PullError;
use crate::{error::WhError, pods::itree::InodeId};
use custom_error::custom_error;
use parking_lot::RwLockReadGuard;

use super::fs_interface::FsInterface;

custom_error! {
    /// Error describing the read syscall
    #[derive(Clone)]
    pub ReadError
    WhError{source: WhError} = "{source}",
    PullError{source: PullError} = "{source}",
    LocalReadFailed{io: Arc<std::io::Error>} = "Local read failed: {io}",
    CantPull = "Unable to pull file",
    NoReadPermission = "The permissions doesn't allow to read",
    NoFileHandle = "The file doesn't have a file handle",
}

impl From<std::io::Error> for ReadError {
    fn from(io: std::io::Error) -> Self {
        Self::LocalReadFailed { io: Arc::new(io) }
    }
}

fn check_file_handle<'a>(
    file_handles: &'a RwLockReadGuard<FileHandleManager>,
    file_handle_id: UUID,
) -> Result<&'a FileHandle, ReadError> {
    match file_handles.handles.get(&file_handle_id) {
        Some(&FileHandle {
            perm: AccessMode::Write,
            direct: _,
            no_atime: _,
            dirty: _,
            ino: _,
            signature: _,
        }) => Err(ReadError::NoReadPermission),
        Some(&FileHandle {
            perm: AccessMode::Execute,
            direct: _,
            no_atime: _,
            dirty: _,
            ino: _,
            signature: _,
        }) => Err(ReadError::NoReadPermission),
        None => Err(ReadError::NoFileHandle),
        Some(file_handle) => Ok(file_handle),
    }
}

impl FsInterface {
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    pub fn get_file_data_sync(
        &self,
        ino: Ino,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, ReadError> {
        match self.network_interface.pull_file_sync(ino)? {
            None => Ok(self.disk.read_file(
                &ITree::read_lock(&self.itree, "read_file")?.get_path_from_inode_id(ino)?,
                offset,
                buf,
            )?),
            Some(data) => {
                let size = data.len().saturating_sub(offset);
                if size > 0 {
                    buf[..size].copy_from_slice(&data[offset..offset + size]);
                }
                Ok(size)
            }
        }
    }

    /// Get or pull the file from storage or network
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    pub fn get_whole_file_sync(&self, ino: Ino) -> Result<File, ReadError> {
        match self.network_interface.pull_file_sync(ino)? {
            None => self
                .get_local_file(ino)
                .map(|o| o.expect("promised by pull_file_sync")),
            Some(data) => Ok(File(data)),
        }
    }

    /// Get locally stored file as-is if it exists without accessing the network
    /// returns Ok(Some(file)) if the file is tracked
    /// returns Ok(None) if the file is not tracked
    pub fn get_local_file(&self, ino: Ino) -> Result<Option<File>, ReadError> {
        let hostname = self.network_interface.hostname()?;
        let mut buf = Vec::new();
        let itree = self.itree.read();
        let size = itree
            .get_inode(ino)
            .and_then(|inode| match &inode.entry {
                FsEntry::File(hosts) => Ok(hosts
                    .contains(&hostname)
                    .then_some(inode.meta.size as usize)),
                FsEntry::Directory(_) => Err(WhError::InodeIsADirectory),
            })?;
        drop(itree);
        if let Some(mut size) = size {
            buf.resize(size, 0);
            size = self.disk.read_file(
                &ITree::read_lock(&self.itree, "read_file")?.get_path_from_inode_id(ino)?,
                0,
                &mut buf[..],
            )?;
            buf.resize(size, 0);
            Ok(Some(File(Arc::new(buf))))
        } else {
            Ok(None)
        }
    }

    pub fn read_file(
        &self,
        file: InodeId,
        offset: usize,
        buf: &mut [u8],
        file_handle: UUID,
    ) -> Result<usize, ReadError> {
        {
            let file_handles = FileHandleManager::read_lock(&self.file_handles, "read")?;
            let _file_handle = check_file_handle(&file_handles, file_handle)?;
        }

        self.get_file_data_sync(file, offset, buf)
    }
}
