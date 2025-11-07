use camino::{FromPathBufError, Iter, Utf8Component, Utf8Path, Utf8PathBuf};
use custom_error::custom_error;
use openat::AsPath;
use std::ffi::CString;
use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::{
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
};

use crate::error::{WhError, WhResult};

custom_error! {pub WhPathError
    NotRelative = "Path is not relative",
    NotValidUtf8 = "Path is not valid UTF-8",
    NotValidPath = "Path is not valid / can't be normalized",
    InvalidOperation = "Operation would compromise WhPath integrity",
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
        if p.is_absolute() {
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
        Self {
            inner: value.into(),
        }
    }
}

impl AsRef<Path> for WhPath {
    fn as_ref(&self) -> &Path {
        self.inner.as_std_path()
    }
}

impl AsRef<str> for WhPath {
    fn as_ref(&self) -> &str {
        self.inner.as_str()
    }
}

impl AsRef<OsStr> for WhPath {
    fn as_ref(&self) -> &OsStr {
        self.inner.as_os_str()
    }
}

impl<'a> AsPath for &'a WhPath {
    type Buffer = CString;
    fn to_path(self) -> Option<CString> {
        CString::new(self.inner.as_str().as_bytes()).ok()
    }
}

impl Debug for WhPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhPath")
            .field("inner", &self.inner)
            .finish()
    }
}

impl Display for WhPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&*self.inner, f)
    }
}

impl WhPath {
    pub fn root() -> Self {
        Self {
            inner: Utf8PathBuf::default(),
        }
    }

    /// Create a relative path from a full path
    ///
    /// Useful to remove the pod's mountpoint from the full file path
    pub fn make_relative<T: AsRef<Path>>(
        full_path: T,
        make_relative_to: T,
    ) -> Result<Self, WhPathError> {
        let relative_path = full_path
            .as_ref()
            .strip_prefix(make_relative_to)
            .map_err(|_| WhPathError::InvalidOperation)?;
        Self::try_from(relative_path)
    }

    pub fn iter(&self) -> Iter<'_> {
        self.inner.iter()
    }

    pub fn push(&mut self, path: impl AsRef<Utf8Path>) -> Result<(), WhPathError> {
        let path = path.as_ref();

        if path.is_absolute() {
            return Err(WhPathError::InvalidOperation);
        } else {
            Ok(self.inner.push(&normalize_utf8path(path)?))
        }
    }

    pub fn join(&self, path: impl AsRef<Utf8Path>) -> Result<Self, WhPathError> {
        let path = path.as_ref();

        if path.is_absolute() {
            return Err(WhPathError::InvalidOperation);
        } else {
            Ok(Self {
                inner: self.inner.join(normalize_utf8path(path)?),
            })
        }
    }
}

/// Normalize a path without accessing the filesystem
///
/// Adapted from:
///
/// https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path(path: impl AsRef<Path>) -> Result<PathBuf, WhPathError> {
    let mut components = path.as_ref().components().peekable();
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

/// Normalize a path without accessing the filesystem
///
/// Adapted from:
///
/// https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_utf8path(path: impl AsRef<Utf8Path>) -> Result<Utf8PathBuf, WhPathError> {
    let mut components = path.as_ref().components().peekable();
    let mut ret = if let Some(c @ Utf8Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        Utf8PathBuf::from(c.as_str())
    } else {
        Utf8PathBuf::new()
    };

    for component in components {
        match component {
            Utf8Component::Prefix(..) => unreachable!(),
            Utf8Component::RootDir => {
                ret.push(component.as_str());
            }
            Utf8Component::CurDir => {}
            Utf8Component::ParentDir => {
                if !ret.pop() {
                    return Err(WhPathError::NotValidPath);
                }
            }
            Utf8Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    Ok(ret)
}

pub fn osstring_to_string(osstr: OsString) -> WhResult<String> {
    osstr.into_string().map_err(|_| WhError::UnsupportedPath)
}

pub fn osstr_to_str(osstr: &OsStr) -> WhResult<&str> {
    osstr.to_str().ok_or(WhError::UnsupportedPath)
}
