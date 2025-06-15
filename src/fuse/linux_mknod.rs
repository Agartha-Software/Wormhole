use crate::pods::filesystem::{
    file_handle::OpenFlags,
    fs_interface::SimpleFileType,
};

const FMODE_EXEC: i32 = 0x20;

impl OpenFlags {
    pub fn from_libc(flags: i32) -> OpenFlags {
        Self {
            no_atime: flags & libc::O_NOATIME != 0,
            direct: flags & libc::O_DIRECT != 0,
            trunc: flags & libc::O_TRUNC != 0,
            exec: flags & FMODE_EXEC != 0,
        }
    }
}

pub fn filetype_from_mode(mode: u32) -> Option<SimpleFileType> {
    let file_type = mode & libc::S_IFMT as u32;

    if file_type == libc::S_IFREG {
        return Some(SimpleFileType::File);
    }
    if file_type == libc::S_IFDIR {
        return Some(SimpleFileType::Directory);
    }
    return None;
}
