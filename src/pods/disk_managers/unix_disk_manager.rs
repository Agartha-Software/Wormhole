use std::{
    ffi::CString,
    io::{Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, RawFd},
        unix::fs::FileExt,
    },
    path::{Path, PathBuf},
};

use openat::Dir;
use tokio::io;

use crate::pods::{itree::SymlinkPath, whpath::WhPath};

use super::DiskManager;

#[derive(Debug)]
pub struct UnixDiskManager {
    handle: Dir,
    path: PathBuf,
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
            path: mount_point.into(),
        })
    }
}

impl UnixDiskManager {
    fn exist(&self, path: &WhPath) -> bool {
        path.as_str().is_empty() || self.handle.metadata(path).is_ok()
    }
}

/// always takes a WhPath and infers the real disk path
impl DiskManager for UnixDiskManager {
    fn stop(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn new_file(&self, path: &WhPath, mode: u16) -> io::Result<()> {
        if self.exist(path) {
            self.handle.remove_file(path)?;
        }
        self.handle.new_file(path, mode.into())?; // TODO look more in c mode_t value
        Ok(())
    }

    fn remove_file(&self, path: &WhPath) -> io::Result<()> {
        self.handle.remove_file(path)
    }

    fn remove_dir(&self, path: &WhPath) -> io::Result<()> {
        self.handle.remove_dir(path)
    }

    fn write_file(&self, path: &WhPath, binary: &[u8], offset: usize) -> io::Result<usize> {
        let file = self.handle.update_file(path, 0o600)?;
        file.write_at(binary, offset as u64) // NOTE - used "as" because into() is not supported
    }

    fn set_file_size(&self, path: &WhPath, size: usize) -> io::Result<()> {
        let file = self.handle.update_file(path, 0o600)?;
        file.set_len(size as u64)
    }

    fn mv_file(&self, path: &WhPath, new_path: &WhPath) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        self.handle.local_rename(path, new_path)
    }

    fn read_file(&self, path: &WhPath, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        let mut file = self.handle.open_file(path)?;
        file.seek(SeekFrom::Start(offset as u64))?;
        file.read(buf)
    }

    fn new_dir(&self, path: &WhPath, permissions: u16) -> io::Result<()> {
        self.handle.create_dir(path, permissions.into()) // TODO look more in c mode_t value
    }

    fn set_permisions(&self, path: &WhPath, permissions: u16) -> std::io::Result<()> {
        let raw_fd: RawFd = self.handle.as_raw_fd();
        let c_string_path =
            CString::new::<&str>(path.as_ref()).expect("panics if there are internal null bytes");

        let ptr: *const i8 = c_string_path.as_ptr();
        let result = unsafe {
            // If we just self.handle.open_file...set_permission, the open flags
            // don't allow to modify the permission on a file where we don't have the permission like a 000
            // This is the only convincing way we found
            libc::fchmodat(raw_fd, ptr, permissions.into(), libc::AT_EMPTY_PATH)
        };
        if result != 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn size_info(&self) -> std::io::Result<super::DiskSizeInfo> {
        todo!()
    }

    fn file_exists(&self, path: &WhPath) -> bool {
        self.handle.open_file(path).is_ok()
    }

fn new_symlink(&self, path: &WhPath, permissions: u16, target: &SymlinkPath) -> std::io::Result<()> {
        let target = target.realize(&self.path);
        self.handle.symlink(path, &target)?;
        self.set_permisions(path, permissions)
    }

    fn remove_symlink(&self, path: &WhPath) -> std::io::Result<()> {
        self.handle.remove_file(path)
    }
}

mod test {
    #[allow(unused_imports)] // clippy not properly detecting this is needed and used
    use crate::pods::disk_managers::unix_disk_manager::UnixDiskManager;

    #[test]
    pub fn test_priv_unix_disk() {
        let temp_dir = assert_fs::TempDir::new().expect("can't create temp dir");
        let disk = UnixDiskManager::new(&temp_dir.path()).expect("creating disk manager");

        assert!(disk.exist(&"".try_into().unwrap()));
    }
}
