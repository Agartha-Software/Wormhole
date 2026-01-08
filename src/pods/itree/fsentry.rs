use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::{WhError, WhResult},
    network::message::Address,
    pods::{filesystem::fs_interface::SimpleFileType, itree::Ino, whpath::WhPath},
};

pub type Hosts = Vec<Address>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SymlinkPath {
    /// Path relative to the symlink file itself
    SymlinkPathRelative(Utf8PathBuf),
    /// Path relative to the WH drive. Not really absolute but emulates absolute symlinks within the WH drive
    SymlinkPathAbsolute(WhPath),
    /// absolute Path pointing outside the WH drive
    SymlinkPathExternal(Utf8PathBuf),
}

impl SymlinkPath {
    pub fn realize(&self, mount: &Path, self_path: &WhPath) -> PathBuf {
        return match self {
            SymlinkPath::SymlinkPathRelative(path) => PathBuf::from_iter([
                mount,
                self_path
                    .parent()
                    .as_ref()
                    .map_or(Path::new(""), |p| p.as_std_path()),
                path.as_std_path(),
            ]),
            SymlinkPath::SymlinkPathAbsolute(path) => mount.join(path),
            SymlinkPath::SymlinkPathExternal(path) => path.into(),
        };
    }
}

impl Display for SymlinkPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymlinkPath::SymlinkPathRelative(path) => f.write_str(path.as_str()),
            SymlinkPath::SymlinkPathAbsolute(path) => write!(f, "//{}", path.as_str()),
            SymlinkPath::SymlinkPathExternal(path) => f.write_str(path.as_str()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EntrySymlink {
    pub target: SymlinkPath,
    pub hint: Option<SimpleFileType>,
}

impl Default for EntrySymlink {
    fn default() -> Self {
        Self {
            target: SymlinkPath::SymlinkPathRelative(".".into()),
            hint: Some(SimpleFileType::Directory),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
/// Should be extended until meeting [fuser::FileType]
pub enum FsEntry {
    File(Hosts),
    Directory(Vec<Ino>),
    Symlink(EntrySymlink),
}

impl FsEntry {
    pub fn new_file() -> Self {
        FsEntry::File(vec![])
    }

    pub fn new_directory() -> Self {
        FsEntry::Directory(vec![])
    }

    pub fn get_filetype(&self) -> SimpleFileType {
        match self {
            FsEntry::File(_) => SimpleFileType::File,
            FsEntry::Directory(_) => SimpleFileType::Directory,
            FsEntry::Symlink(_) => SimpleFileType::Symlink,
        }
    }

    pub fn get_children(&self) -> WhResult<&Vec<Ino>> {
        match self {
            FsEntry::Directory(children) => Ok(children),
            _ => Err(WhError::InodeIsNotADirectory),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::pods::{
        filesystem::fs_interface::SimpleFileType,
        itree::{EntrySymlink, SymlinkPath},
        whpath::WhPath,
    };
    use assert_fs::{
        assert::PathAssert,
        prelude::{PathChild, PathCreateDir},
        TempDir,
    };

    #[test]
    fn test_realize_symlink() {
        // / (temp_dir)
        // |- external_file
        // \- wormhole
        //      |- internal_file
        //      \- folder
        //          |- internal_file2
        //          \- link -> [../internal_file, /external_file, //internal_file, ./internal_file2]

        let temp_dir = TempDir::new().expect("creating temp dir");
        let mount_point = temp_dir.child("wormhole"); //.to_path_buf();

        let folder = WhPath::try_from("folder").expect("checked");
        let link_path = folder.join(&"link".try_into().expect("checked"));

        let internal_file = mount_point.child("internal_file");
        let internal_file2 = mount_point.child(folder).child("internal_file2");

        let external_file = std::path::absolute(&temp_dir)
            .expect(&format!("{:?} should exist", temp_dir.path()))
            .join("external_file");

        mount_point
            .child(&link_path.parent().unwrap_or(WhPath::root()))
            .create_dir_all()
            .expect(&format!(
                "{:?} should be a valid path",
                mount_point.child(&link_path).path()
            ));

        std::fs::write(&internal_file, []).expect(&format!(
            "parent of {:?} should exist",
            internal_file.path()
        ));
        std::fs::write(&internal_file2, []).expect(&format!(
            "parent of {:?} should exist",
            internal_file.path()
        ));
        std::fs::write(&external_file, []).expect(&format!(
            "parent of {:?} should exist",
            external_file.as_path()
        ));

        let relative_link = EntrySymlink {
            target: SymlinkPath::SymlinkPathRelative("../internal_file".into()),
            hint: Some(SimpleFileType::File),
        };

        let relative_link2 = EntrySymlink {
            target: SymlinkPath::SymlinkPathRelative("./internal_file2".into()),
            hint: Some(SimpleFileType::File),
        };

        let absolute_link = EntrySymlink {
            target: SymlinkPath::SymlinkPathAbsolute("internal_file".try_into().unwrap()),
            hint: Some(SimpleFileType::File),
        };

        let external_link = EntrySymlink {
            target: SymlinkPath::SymlinkPathExternal("external_file".into()),
            hint: Some(SimpleFileType::File),
        };

        temp_dir
            .child(relative_link.target.realize(&mount_point, &link_path))
            .assert(predicates::path::exists());

        temp_dir
            .child(relative_link2.target.realize(&mount_point, &link_path))
            .assert(predicates::path::exists());
        temp_dir
            .child(absolute_link.target.realize(&mount_point, &link_path))
            .assert(predicates::path::exists());
        temp_dir
            .child(external_link.target.realize(&mount_point, &link_path))
            .assert(predicates::path::exists());
    }
}
