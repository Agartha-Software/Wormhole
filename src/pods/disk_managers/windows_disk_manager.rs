use std::{
    os::windows::prelude::FileExt,
    path::{Path, PathBuf},
};

use tokio::io;

use crate::winfsp::winfsp_impl::aliased_path;

use super::{DiskManager, DiskSizeInfo};

#[derive(Debug)]
pub struct WindowsDiskManager {
    mount_point: PathBuf, // mountpoint on linux and mirror mountpoint on windows
}

impl WindowsDiskManager {
    pub fn new(mount_point: &Path) -> io::Result<Self> {
        let mount_point = aliased_path(mount_point).map_err(|_| io::ErrorKind::InvalidFilename)?;

        if !mount_point.exists() {
            std::fs::create_dir(&mount_point)?;
        }

        Ok(Self {
            mount_point,
        })
    }
}

/// always takes a WhPath and infers the real disk path
impl DiskManager for WindowsDiskManager {
    fn new_file(&self, path: &Path, permissions: u16) -> io::Result<()> {
        std::fs::File::create(&self.mount_point.join(path))?;
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(&self.mount_point.join(path))
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir(&self.mount_point.join(path))
    }

    fn write_file(&self, path: &Path, binary: &[u8], offset: usize) -> io::Result<usize> {
        return std::fs::File::open(&self.mount_point.join(path))?
            .seek_write(binary, offset as u64);
    }

    fn set_file_size(&self, path: &Path, size: usize) -> io::Result<()> {
        std::fs::File::open(&self.mount_point.join(path))?.set_len(size as u64)
    }

    fn mv_file(&self, path: &Path, new_path: &Path) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        std::fs::rename(
            &self.mount_point.join(path),
            &self.mount_point.join(new_path),
        )
    }

    fn read_file(&self, path: &Path, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        std::fs::File::open(&self.mount_point.join(path))?.seek_read(buf, offset as u64)
    }

    fn new_dir(&self, path: &Path, permissions: u16) -> io::Result<()> {
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

    fn log_arbo(&self, path: &Path) -> std::io::Result<()> {
        todo!()
    }

    fn set_permisions(&self, path: &Path, permissions: u16) -> io::Result<()> {
        Ok(())
    }
}
