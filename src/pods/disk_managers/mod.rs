use std::io;

use std::fmt::Debug;
use std::path::Path;
pub mod windows_disk_manager;
#[cfg(target_os = "linux")]
pub mod unix_disk_manager;

pub struct DiskSizeInfo {
    pub free_size: usize,
    pub total_size: usize,
}

pub trait DiskManager: Send + Sync + Debug {
    fn log_arbo(&self, path: &Path) -> io::Result<()>;

    fn new_file(&self, path: &Path, permissions: u16) -> io::Result<()>;

    fn set_permisions(&self, path: &Path, permissions: u16) -> io::Result<()>;

    fn remove_file(&self, path: &Path) -> io::Result<()>;

    fn remove_dir(&self, path: &Path) -> io::Result<()>;

    fn write_file(&self, path: &Path, binary: &[u8], offset: usize) -> io::Result<usize>;

    fn set_file_size(&self, path: &Path, size: usize) -> io::Result<()>;

    fn mv_file(&self, path: &Path, new_path: &Path) -> io::Result<()>;

    fn read_file(&self, path: &Path, offset: usize, buf: &mut [u8]) -> io::Result<usize>;

    fn new_dir(&self, path: &Path, permissions: u16) -> io::Result<()>;

    fn size_info(&self) -> io::Result<DiskSizeInfo>;
}
