use std::{
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::{WhError, WhResult},
    pods::{
        filesystem::fs_interface::SimpleFileType,
        itree::Ino,
        whpath::{WhPath, WhPathError},
    },
};

pub type Hosts = Vec<PeerId>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, TS)]
pub enum SymlinkPath {
    /// Path relative to the symlink file itself
    SymlinkPathRelative(#[ts(as = "String")] Utf8PathBuf),
    /// Path relative to the WH drive. Not really absolute but emulates absolute symlinks within the WH drive
    SymlinkPathAbsolute(WhPath),
    /// absolute Path pointing outside the WH drive
    SymlinkPathExternal(PathBuf),
}

impl SymlinkPath {
    /// Create a canonical path from a symlink
    pub fn resolve(&self, mount: &Path, self_path: &WhPath) -> PathBuf {
        match self {
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
        }
    }

    /// Create a system path from a symlink
    /// This doesn't resolve the symlink, only handles External vs Absolute distinction
    pub fn realize(&self, mount: &Path) -> PathBuf {
        match self {
            SymlinkPath::SymlinkPathRelative(path) => path.as_std_path().to_owned(),
            SymlinkPath::SymlinkPathAbsolute(path) => mount.join(path),
            SymlinkPath::SymlinkPathExternal(path) => path.into(),
        }
    }
}

impl Display for SymlinkPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymlinkPath::SymlinkPathRelative(path) => f.write_str(path.as_str()),
            SymlinkPath::SymlinkPathAbsolute(path) => write!(f, "//{}", path.as_str()),
            SymlinkPath::SymlinkPathExternal(path) => path.display().fmt(f),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, TS)]
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

impl EntrySymlink {
    /// Err(None) means the 'absolute' test failed
    /// Err(Some(e)) is a parsing error of the target
    pub fn from_absolute<P: AsRef<Path> + ?Sized, Q: AsRef<Path> + ?Sized>(
        target: &P,
        mountpoint: &Q,
    ) -> Result<Self, Option<WhPathError>> {
        let target = target.as_ref();
        let mountpoint = mountpoint.as_ref();
        target.has_root().then_some(()).ok_or(None)?;
        let mut components = normalize(target);

        // gradually matches each component of the mountpoint, untill only the internal portion remains
        for m in mountpoint.iter() {
            if let Some(t) = components.next() {
                if m == t {
                    continue;
                }
            }
            Err(None)?;
        }
        Ok(Self {
            target: SymlinkPath::SymlinkPathAbsolute(
                PathBuf::from_iter(components).try_into().map_err(Some)?,
            ),
            hint: None,
        })
    }

    /// Create a Symlink from a path, considering the mountpoint
    ///
    /// Note: because symlink targets are not checked, any failure results in
    /// creation of a SymlinkPathExternal() regardless of the actual contents
    /// This is because External paths are opque to wormhole and transparent to the OS
    /// If wormhole failed to parse, we can't handle it,
    /// but the os will always treat it as an arbitrary OsString
    ///
    pub fn parse<P: AsRef<Path> + ?Sized, Q: AsRef<Path> + ?Sized>(
        target: &P,
        mountpoint: &Q,
    ) -> Result<Self, Self> {
        let target = target.as_ref();
        let mountpoint = mountpoint.as_ref();
        let external = || Self {
            target: SymlinkPath::SymlinkPathExternal(target.to_path_buf()),
            hint: None,
        };

        if target.has_root() {
            Self::from_absolute(target, mountpoint)
                .or_else(|e| e.map(|_| external()).ok_or_else(external))
        } else {
            Ok(Self {
                target: SymlinkPath::SymlinkPathRelative(
                    Utf8PathBuf::from_os_string(target.into()).map_err(|_| external())?,
                ),
                hint: None,
            })
        }
    }

    pub fn read(&self, mountpoint: &Path) -> PathBuf {
        self.target.realize(mountpoint)
    }
}

fn normalize(path: &Path) -> std::vec::IntoIter<&OsStr> {
    let mut result = vec![];
    for component in path.iter() {
        const DOT: &[u8] = ".".as_bytes();
        const DOTDOT: &[u8] = "..".as_bytes();
        match component.as_encoded_bytes() {
            DOT => {}
            DOTDOT => {
                if !result.is_empty() {
                    result.pop();
                } else {
                    result.push(component)
                }
            }
            component => result.push(unsafe {
                /* direct from .as_encoded_bytes() */
                OsStr::from_encoded_bytes_unchecked(component)
            }),
        }
    }
    result.into_iter()
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
    use camino::Utf8PathBuf;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_parse_symlink() {
        let mountpoint = Path::new("/mountpoint");
        let relative = Path::new("../file");
        let absolute = mountpoint.join("folder/file");
        let external = Path::new("/tmp/file");

        let symlink_relative =
            EntrySymlink::parse(&relative, mountpoint).expect("parsing relative symlink");
        let symlink_relative_expected = EntrySymlink {
            target: SymlinkPath::SymlinkPathRelative(Utf8PathBuf::from("../file")),
            hint: None,
        };
        assert_eq!(symlink_relative, symlink_relative_expected);

        let symlink_absolute =
            EntrySymlink::parse(&absolute, mountpoint).expect("parsing absolute symlink");
        let symlink_absolute_expected = EntrySymlink {
            target: SymlinkPath::SymlinkPathAbsolute(
                WhPath::try_from("folder/file").expect("valid WhPath"),
            ),
            hint: None,
        };
        assert_eq!(symlink_absolute, symlink_absolute_expected);

        let symlink_external = EntrySymlink::parse(&external, mountpoint)
            .expect_err("parsing external symlink yields Err(Self)");
        let symlink_external_expected = EntrySymlink {
            target: SymlinkPath::SymlinkPathExternal(PathBuf::from("/tmp/file")),
            hint: None,
        };
        assert_eq!(symlink_external, symlink_external_expected);
    }

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
            .unwrap_or_else(|e| panic!("{:?} should exist: {e:?}", temp_dir.path()))
            .join("external_file");

        mount_point
            .child(link_path.parent().unwrap_or(WhPath::root()))
            .create_dir_all()
            .unwrap_or_else(|e| {
                panic!(
                    "{:?} should be a valid path: {e:?}",
                    mount_point.child(&link_path).path()
                )
            });

        std::fs::write(&internal_file, [])
            .unwrap_or_else(|e| panic!("parent of {:?} should exist: {e:?}", internal_file.path()));
        std::fs::write(&internal_file2, []).unwrap_or_else(|e| {
            panic!("parent of {:?} should exist: {e:?}", internal_file2.path())
        });
        std::fs::write(&external_file, []).unwrap_or_else(|e| {
            panic!(
                "parent of {:?} should exist: {e:?}",
                external_file.as_path()
            )
        });

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
            .child(relative_link.target.resolve(&mount_point, &link_path))
            .assert(predicates::path::exists());

        temp_dir
            .child(relative_link2.target.resolve(&mount_point, &link_path))
            .assert(predicates::path::exists());
        temp_dir
            .child(absolute_link.target.resolve(&mount_point, &link_path))
            .assert(predicates::path::exists());
        temp_dir
            .child(external_link.target.resolve(&mount_point, &link_path))
            .assert(predicates::path::exists());
    }
}
