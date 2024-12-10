use std::ffi::OsString;

use windows_permissions::{constants::SecurityInformation, LocalBox, SecurityDescriptor, WindowsSecure};
use winfsp::U16CString;

use super::FolderHandle;



pub struct WindowsFolderHandle {
    name: OsString,
}

impl WindowsFolderHandle {
    pub fn security_descriptor(&self, sec_info: SecurityInformation) -> std::io::Result<LocalBox<SecurityDescriptor>> {
        self.name.security_descriptor(sec_info)
    }

}

impl FolderHandle for WindowsFolderHandle {
    fn open_file(&self, path: &std::path::Path) -> std::io::Result<std::fs::File> {
        todo!()
    }

    fn new_file(&self, path: &std::path::Path, mode: i32) -> std::io::Result<std::fs::File> {
        todo!()
    }

    fn write_file(&self, path: &std::path::Path, mode: i32) -> std::io::Result<std::fs::File> {
        todo!()
    }

    fn remove_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        todo!()
    }

    fn create_dir(&self, path: &std::path::Path, mode: i32) -> std::io::Result<std::fs::File> {
        todo!()
    }

    fn metadata(&self, path: &std::path::Path) -> std::io::Result<std::fs::Metadata> {
        todo!()
    }
}