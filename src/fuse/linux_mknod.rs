use crate::pods::filesystem::fs_interface::SimpleFileType;

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
