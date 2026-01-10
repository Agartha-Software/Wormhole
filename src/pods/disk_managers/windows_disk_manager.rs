use std::{
    os::windows::prelude::FileExt,
    path::{Path, PathBuf},
};

use tokio::io;

use crate::{
    pods::{filesystem::fs_interface::SimpleFileType, itree::EntrySymlink, whpath::WhPath},
    winfsp::winfsp_impl::aliased_path,
};

use super::{DiskManager, DiskSizeInfo};

#[derive(Debug)]
pub struct WindowsDiskManager {
    mount_point: PathBuf, // (aliased original location)
    original_location: PathBuf,
    stopped: bool,
}

impl WindowsDiskManager {
    /// On windows, the original dir is moved from "name" to ".name"
    pub fn new(mount_point: &Path) -> io::Result<Self> {
        let system_mount_point =
            aliased_path(mount_point).map_err(|_| io::ErrorKind::InvalidFilename)?;

        if system_mount_point.exists() {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "System virtual mountpoint already existing (path/.mountpoint). Please delete it to create a pod here."));
        }
        std::fs::rename(mount_point, &system_mount_point)?;

        Ok(Self {
            mount_point: system_mount_point,
            original_location: mount_point.to_owned(),
            stopped: false,
        })
    }
}

impl Drop for WindowsDiskManager {
    fn drop(&mut self) {
        if !self.stopped {
            let _ = std::fs::rename(&self.mount_point, &self.original_location).inspect_err(|e| log::error!("WindowsDiskManager was unable to move the directory to its initial location (on drop): {e}"));
        }
    }
}

/// always takes a WhPath and infers the real disk path
impl DiskManager for WindowsDiskManager {
    fn stop(&mut self) -> io::Result<()> {
        log::trace!("Stop of WindowsDiskManager");

        self.stopped = true;
        std::fs::rename(&self.mount_point, &self.original_location)?;
        Ok(())
    }

    fn new_file(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        std::fs::File::create(&self.mount_point.join(path))?;
        Ok(())
    }

    fn remove_file(&self, path: &WhPath) -> io::Result<()> {
        std::fs::remove_file(&self.mount_point.join(path))
    }

    fn remove_dir(&self, path: &WhPath) -> io::Result<()> {
        std::fs::remove_dir(&self.mount_point.join(path))
    }

    fn write_file(&self, path: &WhPath, binary: &[u8], offset: usize) -> io::Result<usize> {
        std::fs::File::options()
            .write(true)
            .open(&self.mount_point.join(path))?
            .seek_write(binary, offset as u64)
    }

    fn set_file_size(&self, path: &WhPath, size: usize) -> io::Result<()> {
        std::fs::File::options()
            .write(true)
            .open(&self.mount_point.join(path))?
            .set_len(size as u64)
    }

    fn mv_file(&self, path: &WhPath, new_path: &WhPath) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        std::fs::rename(
            &self.mount_point.join(path),
            &self.mount_point.join(new_path),
        )
    }

    fn read_file(&self, path: &WhPath, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        std::fs::File::open(&self.mount_point.join(path))?.seek_read(buf, offset as u64)
    }

    fn new_dir(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        std::fs::create_dir(&self.mount_point.join(path))
    }

    fn size_info(&self) -> std::io::Result<super::DiskSizeInfo> {
        Ok(DiskSizeInfo {
            free_size: fs2::free_space(&self.mount_point)? as usize,
            total_size: fs2::total_space(&self.mount_point)? as usize,
        })
    }

    fn set_permisions(&self, _path: &WhPath, _permissions: u16) -> io::Result<()> {
        log::warn!("permissions not supported on windows");
        Ok(())
    }

    fn file_exists(&self, path: &WhPath) -> bool {
        std::fs::exists(&self.mount_point.join(path)).unwrap_or(false)
    }

    fn new_symlink(
        &self,
        path: &WhPath,
        permissions: u16,
        link: &EntrySymlink,
    ) -> std::io::Result<()> {
        // replace with a dummy file or folder
        match link.hint {
            Some(SimpleFileType::Directory) => self.new_dir(path, permissions),
            _ => self.new_file(path, permissions),
        }
    }

    fn remove_symlink(&self, path: &WhPath) -> std::io::Result<()> {
        // replaced with a dummy file or folder
        let path = self.mount_point.join(path);
        if path.is_dir() {
            std::fs::remove_dir(&path)
        } else if path.is_file() {
            std::fs::remove_file(&path)
        } else {
            Ok(())
        }
    }
}
