use std::{
    ffi::CString,
    io::Read,
    os::{
        fd::{AsRawFd, RawFd},
        unix::fs::FileExt,
    },
    path::Path,
    path::PathBuf,
};

use openat::Dir;
use tokio::io;

use super::DiskManager;

#[derive(Debug)]
pub struct UnixDiskManager {
    handle: Dir,
    mount_point: PathBuf,
}

impl UnixDiskManager {
    pub fn new(mount_point: &Path) -> io::Result<Self> {
        // /!\
        // /!\

        unsafe { libc::umask(0o000) }; //TODO: Remove when handling permissions

        // /!\
        // /!\

        std::fs::create_dir(mount_point).or_else(|e| {
            (e.kind() == io::ErrorKind::AlreadyExists)
                .then_some(())
                .ok_or(e)
        })?;

        Ok(Self {
            handle: Dir::open(mount_point)?,
            mount_point: mount_point.clone(),
        })
    }
}

impl UnixDiskManager {
    fn exist(&self, path: &Path) -> bool {
        self.handle.metadata(path).is_ok()
    }
}

/// always takes a WhPath and infers the real disk path
impl DiskManager for UnixDiskManager {
    /// Very simple util to log the content of a folder locally
    fn log_arbo(&self, path: &Path) -> io::Result<()> {
        // TODO - unused function ?
        let dirs = self.handle.list_dir(path)?;
        for dir in dirs {
            match dir {
                Ok(entry) => log::debug!("|{:?} => {:?}|", entry.file_name(), entry.simple_type()),
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    fn new_file(&self, path: &Path, mode: u16) -> io::Result<()> {
        if self.exist(path) {
            self.handle.remove_file(path)?;
        }
        self.handle.new_file(path, mode.into())?; // TODO look more in c mode_t value
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.handle.remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.handle.remove_dir(path)
    }

    fn write_file(&self, path: &Path, binary: &[u8], offset: usize) -> io::Result<usize> {
        let file = self.handle.append_file(path, 0o600)?; //  [openat::update_file]?
        Ok(file.write_at(&binary, offset as u64)?) // NOTE - used "as" because into() is not supported
    }

    fn set_file_size(&self, path: &Path, size: usize) -> io::Result<()> {
        let file = self.handle.append_file(path, 0o600)?;
        file.set_len(size as u64)
    }

    fn mv_file(&self, path: &Path, new_path: &Path) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        self.handle.local_rename(path, new_path)
    }

    fn read_file(&self, path: &Path, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        let file = self.handle.open_file(path)?;
        let mut read = file
            .bytes()
            .skip(offset)
            .take(buf.len())
            .map_while(|b| b.ok())
            .collect::<Vec<u8>>();
        let read_len = read.len();
        buf[0..read_len].swap_with_slice(&mut read);
        Ok(read_len)
    }

    fn new_dir(&self, path: &Path, permissions: u16) -> io::Result<()> {
        self.handle.create_dir(path, permissions.into()) // TODO look more in c mode_t value
    }

    fn set_permisions(&self, path: &Path, permissions: u16) -> std::io::Result<()> {
        let raw_fd: RawFd = self.handle.as_raw_fd();
        let c_string_path = CString::new(path.to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "PathBuf -> &str failed",
        ))?)
        .expect("panics if there are internal null bytes");

        let ptr: *const i8 = c_string_path.as_ptr();
        unsafe {
            // If we just self.handle.open_file...set_permission, the open flags
            // don't allow to modify the permission on a file where we don't have the permission like a 000
            // This is the only convincing way we found
            libc::fchmodat(raw_fd, ptr, permissions.into(), 0);
        }
        Ok(())
    }

    fn size_info(&self) -> std::io::Result<super::DiskSizeInfo> {
        todo!()
    }
}
