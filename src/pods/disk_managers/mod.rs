use std::io;

use std::fmt::Debug;
use std::path::PathBuf;
pub mod dummy_disk_manager;
#[cfg(target_os = "linux")]
pub mod unix_disk_manager;

pub struct DiskSizeInfo {
    pub free_size: usize,
    pub total_size: usize,
}

pub trait DiskManager: Send + Sync + Debug {
    fn log_arbo(&self, path: &PathBuf) -> io::Result<()>;

    fn new_file(&self, path: &PathBuf, permissions: u16) -> io::Result<()>;

    fn set_permisions(&self, path: &PathBuf, permissions: u16) -> io::Result<()>;

    fn remove_file(&self, path: &PathBuf) -> io::Result<()>;

    fn remove_dir(&self, path: &PathBuf) -> io::Result<()>;

    fn write_file(&self, path: &PathBuf, binary: &[u8], offset: usize) -> io::Result<usize>;

    fn set_file_size(&self, path: &PathBuf, size: usize) -> io::Result<()>;

    fn mv_file(&self, path: &PathBuf, new_path: &PathBuf) -> io::Result<()>;

    fn read_file(&self, path: &PathBuf, offset: usize, buf: &mut [u8]) -> io::Result<usize>;

    fn new_dir(&self, path: &PathBuf, permissions: u16) -> io::Result<()>;

    fn size_info(&self) -> io::Result<DiskSizeInfo>;
}
