use std::{fs::File, io::Write, os::unix::fs::FileExt};

use log::debug;
use openat::Dir;
use tokio::io;

use super::whpath::WhPath;

pub struct DiskManager {
    handle: Dir,
    mount_point: WhPath, // mountpoint on linux and mirror mountpoint on windows
}

/// always takes a WhPath and infers the real disk path
impl DiskManager {
    pub fn new(mount_point: WhPath) -> io::Result<Self> {
        debug!("DiskManager opening dir at {}", mount_point);
        Ok(Self {
            handle: Dir::open(mount_point.clone())?,
            mount_point,
        })
    }

    pub fn new_file(&self, path: &WhPath) -> io::Result<File> {
        let path: WhPath = self.mount_point.join(path);
        self.handle.new_file(path, 0o644) // TODO look more in c mode_t value
    }

    pub fn remove_file(&self, path: &WhPath) -> io::Result<()> {
        let path = self.mount_point.join(path);
        self.handle.remove_file(path)
    }

    pub fn remove_dir(&self, path: &WhPath) -> io::Result<()> {
        let path = self.mount_point.join(path);
        self.handle.remove_dir(path)
    }

    pub fn write_file(&self, path: &WhPath, binary: Vec<u8>, offset: u64) -> io::Result<u64> {
        let path = self.mount_point.join(path);
        let file = self.handle.write_file(path, 0o600)?;
        Ok(file.write_at(&binary, offset)? as u64) // NOTE - used "as" because into() is not supported
    }

    pub fn read_file(&self, path: &WhPath, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        let path = self.mount_point.join(path);
        let file = self.handle.open_file(path)?;
        let mut buf = Vec::with_capacity(
            len.try_into()
                .expect("disk_manager::read_file: can't convert u64 to usize"),
        );

        file.read_exact_at(&mut buf, offset)?;
        Ok(buf)
    }

    pub fn new_dir(&self, path: &WhPath) -> io::Result<()> {
        let path = self.mount_point.join(path);
        self.handle.create_dir(path, 0o644) // TODO look more in c mode_t value
    }
}
