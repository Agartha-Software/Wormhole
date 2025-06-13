use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    error::{WhError, WhResult},
    pods::arbo::LOCK_TIMEOUT,
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
    Execute,
}

pub type UUID = u64;

#[derive(Debug)]
pub struct FileHandle {
    pub perm: AccessMode,
    pub no_atime: bool,
    pub direct: bool,
}

#[derive(Debug)]
pub struct FileHandleManager {
    pub handles: HashMap<UUID, FileHandle>,
    pub hasher: DefaultHasher,
}

impl FileHandleManager {
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
            hasher: DefaultHasher::new(),
        }
    }

    pub fn new_uuid(&mut self) -> UUID {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("We are earlier than 1970");
        since_the_epoch.hash(&mut self.hasher);
        self.hasher.finish()
    }

    pub fn insert_new_file_handle(&mut self, flags: i32, perm: AccessMode) -> WhResult<UUID> {
        let direct = flags & libc::O_DIRECT != 0;
        let no_atime = flags & libc::O_NOATIME != 0;

        let uuid = self.new_uuid();
        self.handles.insert(
            uuid,
            FileHandle {
                perm,
                direct,
                no_atime,
            },
        );
        Ok(uuid)
    }

    pub fn read_lock<'a>(
        file_handle_manager: &'a Arc<RwLock<FileHandleManager>>,
        called_from: &'a str,
    ) -> WhResult<RwLockReadGuard<'a, FileHandleManager>> {
        file_handle_manager
            .try_read_for(LOCK_TIMEOUT)
            .ok_or(WhError::WouldBlock {
                called_from: called_from.to_owned(),
            })
    }

    pub fn write_lock<'a>(
        file_handle_manager: &'a Arc<RwLock<FileHandleManager>>,
        called_from: &'a str,
    ) -> WhResult<RwLockWriteGuard<'a, FileHandleManager>> {
        file_handle_manager
            .try_write_for(LOCK_TIMEOUT)
            .ok_or(WhError::WouldBlock {
                called_from: called_from.to_owned(),
            })
    }
}
