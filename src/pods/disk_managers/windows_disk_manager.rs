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
        return std::fs::File::open(&self.mount_point.join(path))?
            .seek_write(binary, offset as u64);
    }

    fn set_file_size(&self, path: &WhPath, size: usize) -> io::Result<()> {
        std::fs::File::open(&self.mount_point.join(path))?.set_len(size as u64)
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
        // REVIEW
        // Tries to find the disk that matches the mount point
        // On linux, one disk can be mounted on /, and the other on /disk, and wh on /disk/wh
        // Thus we only keep the disk with the longest mount point matching our mount point path
        // Not sure if this is also the case on windows, so maybe this can be removed.
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let disk = disks
            .into_iter()
            .filter(|disk| self.mount_point.starts_with(disk.mount_point())) // mount point matches disk path
            .fold((0, None), |(candidate_len, candidate), disk| {
                // Keeping the longest mount point
                let current_path_len = disk.mount_point().components().count();
                if let Some(candidate) = candidate {
                    if current_path_len > candidate_len {
                        (current_path_len, Some(disk))
                    } else {
                        (candidate_len, Some(candidate))
                    }
                } else {
                    (current_path_len, Some(disk))
                }
            })
            .1
            .ok_or(io::ErrorKind::Other)
            .inspect_err(|_| log::error!("size_info: disk should be found at this point."))?;

        Ok(DiskSizeInfo {
            free_size: disk.available_space() as usize,
            total_size: disk.total_space() as usize,
        })
    }

    fn set_permisions(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        log::warn!("permissions not supported on windows");
        Ok(())
    }

    fn file_exists(&self, path: &WhPath) -> bool {
        std::fs::exists(&self.mount_point.join(path)).unwrap_or(false)
    }
}
