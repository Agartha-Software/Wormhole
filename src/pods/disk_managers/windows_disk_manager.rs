use std::{
    os::windows::prelude::FileExt,
    path::{Path, PathBuf},
};

use tokio::io;

use crate::{pods::whpath::WhPath, winfsp::winfsp_impl::aliased_path};

use super::{DiskManager, DiskSizeInfo};

#[derive(Debug)]
pub struct WindowsDiskManager {
    mount_point: PathBuf, // mountpoint on linux and mirror mountpoint on windows
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
        })
    }
}

impl Drop for WindowsDiskManager {
    fn drop(&mut self) {
        log::debug!("Drop of WindowsDiskManager");
        let aliased = aliased_path(&self.mount_point).unwrap();

        if std::fs::metadata(&aliased).is_ok() {
            log::debug!("moving from {:?} to {:?} ...", &aliased, &self.mount_point);
            let _ = std::fs::rename(aliased, &self.mount_point);
        }
    }
}

/// always takes a WhPath and infers the real disk path
impl DiskManager for WindowsDiskManager {
    fn new_file(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        std::fs::File::create(&self.mount_point.join(path))
            .inspect_err(|e| log::error!("WDM::new_file Error: {e}"))?;
        Ok(())
    }

    fn remove_file(&self, path: &WhPath) -> io::Result<()> {
        std::fs::remove_file(&self.mount_point.join(path))
            .inspect_err(|e| log::error!("WDM::remove_file Error: {e}"))
    }

    fn remove_dir(&self, path: &WhPath) -> io::Result<()> {
        std::fs::remove_dir(&self.mount_point.join(path))
            .inspect_err(|e| log::error!("WDM::remove_dir Error: {e}"))
    }

    fn write_file(&self, path: &WhPath, binary: &[u8], offset: usize) -> io::Result<usize> {
        std::fs::File::options()
            .write(true)
            .open(&self.mount_point.join(path))?
            .seek_write(binary, offset as u64)
            .inspect_err(|e| log::error!("WDM::write_file Error: {e}"))
    }

    fn set_file_size(&self, path: &WhPath, size: usize) -> io::Result<()> {
        std::fs::File::options()
            .write(true)
            .open(&self.mount_point.join(path))?
            .set_len(size as u64)
            .inspect_err(|e| log::error!("WDM::set_file_size Error: {e}"))
    }

    fn mv_file(&self, path: &WhPath, new_path: &WhPath) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        std::fs::rename(
            &self.mount_point.join(path),
            &self.mount_point.join(new_path),
        )
        .inspect_err(|e| log::error!("WDM::mv_file Error: {e}"))
    }

    fn read_file(&self, path: &WhPath, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        std::fs::File::open(&self.mount_point.join(path))?
            .seek_read(buf, offset as u64)
            .inspect_err(|e| log::error!("WDM::read_file Error: {e}"))
    }

    fn new_dir(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        std::fs::create_dir(&self.mount_point.join(path))
            .inspect_err(|e| log::error!("WDM::new_dir Error: {e}"))
    }

    fn size_info(&self) -> std::io::Result<super::DiskSizeInfo> {
        Ok(DiskSizeInfo {
            free_size: fs2::free_space(&self.mount_point)? as usize,
            total_size: fs2::total_space(&self.mount_point)? as usize,
        })
        .inspect_err(|e| log::error!("WDM::size_info Error: {e}"))
    }

    fn set_permisions(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        log::warn!("permissions not supported on windows");
        Ok(())
    }

    fn file_exists(&self, path: &WhPath) -> bool {
        std::fs::exists(&self.mount_point.join(path)).unwrap_or(false)
    }
}
