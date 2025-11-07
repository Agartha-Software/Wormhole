use std::{
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
};

use camino::{FromPathBufError, Utf8Path, Utf8PathBuf};
use custom_error::custom_error;

custom_error! {pub WhPathError
    NotRelative = "Can't get folder name",
    NotValidUtf8 = "Path is not valid utf8",
    NotValidPath = "Path is not valid / can't be normalized"
}

impl From<FromPathBufError> for WhPathError {
    fn from(_: FromPathBufError) -> Self {
        Self::NotValidUtf8
    }
}

pub struct WhPath {
    inner: Utf8PathBuf,
}

impl TryFrom<PathBuf> for WhPath {
    type Error = WhPathError;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        if !p.is_relative() {
            return Err(Self::Error::NotRelative);
        }
        let normalized_path = normalize_path(&p)?;
        let valid_path = Utf8PathBuf::try_from(normalized_path)?;
        Ok(Self { inner: valid_path })
    }
}

impl TryFrom<&Path> for WhPath {
    type Error = WhPathError;

    fn try_from(p: &Path) -> Result<Self, Self::Error> {
        Self::try_from(p.to_path_buf())
    }
}

impl TryFrom<OsString> for WhPath {
    type Error = WhPathError;

    fn try_from(p: OsString) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(p))
    }
}

impl TryFrom<&OsStr> for WhPath {
    type Error = WhPathError;

    fn try_from(p: &OsStr) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(p))
    }
}

impl TryFrom<String> for WhPath {
    type Error = WhPathError;

    fn try_from(p: String) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(p))
    }
}

impl TryFrom<&str> for WhPath {
    type Error = WhPathError;

    fn try_from(p: &str) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(p))
    }
}

impl From<Utf8PathBuf> for WhPath {
    fn from(value: Utf8PathBuf) -> Self {
        Self { inner: value }
    }
}

impl From<&Utf8Path> for WhPath {
    fn from(value: &Utf8Path) -> Self {
        Self { inner: value.into() }
    }
}

impl WhPath {
    fn root() -> Self {
        Self { inner: Utf8PathBuf::default() }
    }

    fn to_absolute(&self, absolute: &Utf8Path) -> Utf8PathBuf {
        absolute.join(&self.inner)
    }
}

/// Normalize a path without accessing the filesystem
///
/// Adapted from:
///
/// https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path(path: &Path) -> Result<PathBuf, WhPathError> {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !ret.pop() {
                    return Err(WhPathError::NotValidPath);
                }
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    Ok(ret)
}
