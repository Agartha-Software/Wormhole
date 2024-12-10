#[cfg(target_os = "linux")]
pub mod unix;

#[cfg(target_os = "windows")]
pub mod windows;

use std::{fs::{File, Metadata}, io, path::Path};

pub trait FolderHandle {
    // fn open(path: &Path) -> io::Result<Box<Self>>;

    fn open_file(&self, path: &Path) -> io::Result<File>;
    
    fn new_file(&self, path: &Path, mode: i32) -> io::Result<File>;
    
    fn write_file(&self, path: &Path, mode: i32) -> io::Result<File>;
    
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    
    fn create_dir(&self, path: &Path, mode: i32) -> io::Result<File>;

    fn metadata(&self, path: &Path) -> io::Result<Metadata>;
}