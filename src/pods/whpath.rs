use camino::{FromPathBufError, Iter, Utf8Path, Utf8PathBuf};
use custom_error::custom_error;
#[cfg(target_os = "linux")]
use openat::AsPath;
#[cfg(target_os = "linux")]
use std::ffi::CString;
use std::fmt::{Debug, Display};
use std::io;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::error::WhResult;

custom_error! {pub WhPathError
    NotRelative = "Path is not relative",
    NotValidUtf8 = "Path is not valid UTF-8",
    NotNormalized = "Path is not normal",
    InvalidOperation = "Operation would compromise WhPath integrity",
}

impl From<FromPathBufError> for WhPathError {
    fn from(_: FromPathBufError) -> Self {
        Self::NotValidUtf8
    }
}

impl WhPathError {
    pub fn to_io(&self) -> io::Error {
        match self {
            WhPathError::NotRelative => {
                io::Error::new(io::ErrorKind::Other, "WhPath: path is not relative")
            }
            WhPathError::NotValidUtf8 => {
                io::Error::new(io::ErrorKind::InvalidData, "WhPath: not UTF-8")
            }
            WhPathError::NotNormalized => {
                io::Error::new(io::ErrorKind::InvalidFilename, "WhPath: not normalized")
            }
            WhPathError::InvalidOperation => io::Error::new(
                io::ErrorKind::Other,
                "Operation would compromise WhPath integrity",
            ),
        }
    }
}

pub struct WhPath {
    inner: Utf8PathBuf,
}

impl PartialEq for WhPath {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl TryFrom<PathBuf> for WhPath {
    type Error = WhPathError;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        if p.is_absolute() {
            return Err(Self::Error::NotRelative);
        }
        if p.components()
            .any(|c| c.as_os_str() == ".." || c.as_os_str() == ".")
        {
            return Err(Self::Error::NotNormalized);
        }
        Ok(Self {
            inner: Utf8PathBuf::try_from(p)?,
        })
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

#[cfg(target_os = "windows")]
impl TryFrom<&winfsp::U16CStr> for WhPath {
    type Error = WhPathError;

    fn try_from(value: &winfsp::U16CStr) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string().map_err(|_| WhPathError::NotValidUtf8)?)
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

impl AsRef<Utf8Path> for WhPath {
    fn as_ref(&self) -> &Utf8Path {
        &self.inner
    }
}

#[cfg(target_os = "linux")]
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
            Ok(self.inner.push(path))
        }
    }

    pub fn join(&self, path: impl AsRef<Utf8Path>) -> Result<Self, WhPathError> {
        let path = path.as_ref();

        if path.is_absolute() {
            return Err(WhPathError::InvalidOperation);
        } else {
            Ok(Self {
                inner: self.inner.join(path),
            })
        }
    }
}

/*

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
                    return Err(WhPathError::NotNormalized);
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
                    return Err(WhPathError::NotNormalized);
                }
            }
            Utf8Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    Ok(ret)
}
*/

pub fn osstring_to_string(osstr: OsString) -> WhResult<String> {
    osstr
        .into_string()
        .map_err(|_| WhPathError::NotValidUtf8.into())
}

pub fn osstr_to_str(osstr: &OsStr) -> WhResult<&str> {
    osstr.to_str().ok_or(WhPathError::NotValidUtf8.into())
}
