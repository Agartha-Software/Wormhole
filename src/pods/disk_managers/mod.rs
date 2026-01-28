use std::io;

use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::pods::itree::EntrySymlink;
use crate::pods::whpath::WhPath;
#[cfg(target_os = "linux")]
pub mod unix_disk_manager;
#[cfg(target_os = "windows")]
pub mod windows_disk_manager;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DiskSizeInfo {
    pub free_size: usize,
    pub total_size: usize,
    pub files: u64, // Total number of inodes (files)
    pub ffree: u64, // Free inodes available
    pub bsize: u32, // Block size in bytes
}

pub trait DiskManager: Send + Sync + Debug {
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

    fn file_exists(&self, path: &WhPath) -> bool;

    fn stop(&mut self) -> io::Result<()>;

    fn new_symlink(&self, path: &WhPath, permissions: u16, link: &EntrySymlink) -> io::Result<()>;

    fn remove_symlink(&self, path: &WhPath) -> io::Result<()>;
}
