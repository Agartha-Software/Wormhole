use std::io;

use super::whpath::WhPath;
use std::fmt::Debug;
pub mod dummy_disk_manager;
#[cfg(target_os = "linux")]
pub mod unix_disk_manager;
//#[cfg(target_os = "windows")]
//pub mod windows_disk_manager;

pub struct DiskSizeInfo {
    pub free_size: usize,
    pub total_size: usize,
}

pub trait DiskManager: Send + Sync + Debug {
    fn log_arbo(&self, path: &WhPath) -> io::Result<()>;

    fn new_file(&self, path: &WhPath, permissions: u16) -> io::Result<()>;

    fn set_permisions(&self, path: &WhPath, permissions: u16) -> io::Result<()>;

    fn remove_file(&self, path: &WhPath) -> io::Result<()>;

    fn remove_dir(&self, path: &WhPath) -> io::Result<()>;

    fn write_file(&self, path: &WhPath, binary: &[u8], offset: usize) -> io::Result<usize>;

    fn set_file_size(&self, path: &WhPath, size: usize) -> io::Result<()>;

    fn mv_file(&self, path: &WhPath, new_path: &WhPath) -> io::Result<()>;

    fn read_file(&self, path: &WhPath, offset: usize, buf: &mut [u8]) -> io::Result<usize>;

    fn new_dir(&self, path: &WhPath, permissions: u16) -> io::Result<()>;

    fn size_info(&self) -> io::Result<DiskSizeInfo>;
}
