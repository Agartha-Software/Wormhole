use std::{ffi::CString, fs::File, io::Read, os::unix::fs::FileExt};

use openat::{AsPath, Dir};
use tokio::io;

use super::whpath::WhPath;

impl AsPath for WhPath {
    type Buffer = CString;

    fn to_path(self) -> Option<Self::Buffer> {
        CString::new(self.inner).ok()
    }
}

#[derive(Debug)]
pub struct DiskManager {
    handle: Dir,
    mount_point: WhPath, // mountpoint on linux and mirror mountpoint on windows
}

/// always takes a WhPath and infers the real disk path
impl DiskManager {
    pub fn new(mount_point: WhPath) -> io::Result<Self> {
        // /!\
        // /!\

        unsafe { libc::umask(0o000) }; //TODO: Remove when handling permissions

        // /!\
        // /!\

        Ok(Self {
            handle: Dir::open(mount_point.clone())?,
            mount_point,
        })
    }

    // Very simple util to log the content of a folder locally
    pub fn log_arbo(&self, path: String) -> io::Result<()> {
        let dirs = self.handle.list_dir(path)?;
        for dir in dirs {
            match dir {
                Ok(entry) => log::debug!("|{:?} => {:?}|", entry.file_name(), entry.simple_type()),
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn new_file(&self, path: WhPath, permissions: u16) -> io::Result<File> {
        self.handle
            .new_file(path.set_relative(), permissions as libc::mode_t) // TODO look more in c mode_t value
    }

    #[must_use]
    pub fn remove_file(&self, path: WhPath) -> io::Result<()> {
        self.handle.remove_file(path.set_relative())
    }

    #[must_use]
    pub fn remove_dir(&self, path: WhPath) -> io::Result<()> {
        self.handle.remove_dir(path.set_relative())
    }

    pub fn write_file(&self, path: WhPath, binary: &[u8], offset: u64) -> io::Result<u64> {
        let file = self.handle.append_file(path.set_relative(), 0o600)?;
        Ok(file.write_at(&binary, offset)? as u64) // NOTE - used "as" because into() is not supported
    }

    pub fn set_file_size(&self, path: WhPath, size: u64) -> io::Result<()> {
        let file = self.handle.write_file(path.set_relative(), 0o600)?;
        file.set_len(size)
    }

    pub fn mv_file(&self, path: WhPath, new_path: WhPath) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        self.handle
            .local_rename(path.set_relative(), new_path.set_relative())
    }

    pub fn read_file(&self, path: WhPath, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        let file = std::io::BufReader::new(self.handle.open_file(path.set_relative())?);
        let mut buf = Vec::<u8>::new();
        buf.splice(
            0..0,
            file.bytes()
                .skip(offset as usize)
                .take(len as usize)
                .map_while(|b| b.ok()),
        );

        Ok(buf)
    }

    pub fn new_dir(&self, path: WhPath, permissions: u16) -> io::Result<()> {
        self.handle
            .create_dir(path.set_relative(), permissions as libc::mode_t)
    }
}
