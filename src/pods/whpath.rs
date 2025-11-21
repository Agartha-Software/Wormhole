use camino::{FromPathBufError, Iter, Utf8Path, Utf8PathBuf};
use custom_error::custom_error;
#[cfg(target_os = "linux")]
use openat::AsPath;
#[cfg(target_os = "linux")]
use std::ffi::CString;
use std::fmt::{Debug, Display};
use std::io;
use std::ops::Deref;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::error::WhResult;
use crate::pods::arbo::Inode;

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
        is_valid_for_whpath(p)?;

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

impl From<&Inode> for WhPath {
    /// From Inode is UNCHECKED as inodes names should already be correct
    fn from(value: &Inode) -> Self {
        let p: Utf8PathBuf = value.name.clone().into();

        Self { inner: p }
    }
}

#[cfg(target_os = "windows")]
impl TryFrom<&winfsp::U16CStr> for WhPath {
    type Error = WhPathError;

    fn try_from(value: &winfsp::U16CStr) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string().map_err(|_| WhPathError::NotValidUtf8)?)
    }
}

impl<T> AsRef<T> for WhPath
where
    T: ?Sized,
    Utf8PathBuf: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.inner.as_ref()
    }
}

impl Deref for WhPath {
    type Target = Utf8PathBuf;

    fn deref(&self) -> &Self::Target {
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

    /// Allows for a easy path.typed_ref<T>() instead of a heavy rust type notation of AsRef
    pub fn typed_ref<T>(&self) -> &T
    where
        T: ?Sized,
        Utf8PathBuf: AsRef<T>,
    {
        self.as_ref()
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

    #[cfg(target_os = "windows")]
    /// On Windows, '/' is not considered absolute (see PathBuf impl).
    /// However "\\" (that should be considered relative) is not considered same as "" (also relative)
    ///
    /// Should be used with winfsp that gives paths starting with "\\"
    pub fn from_fake_absolute(path: &winfsp::U16CStr) -> Result<Self, WhPathError> {
        path.to_string()
            .map_err(|_| WhPathError::NotValidUtf8)?
            .trim_start_matches("\\")
            .try_into()
    }

    pub fn iter(&self) -> Iter<'_> {
        self.inner.iter()
    }

    pub fn push(&mut self, path: WhPath) {
        self.inner.push(path.inner);
    }

    pub fn join(&self, path: &WhPath) -> Self {
        Self {
            inner: self.inner.join(&path.inner),
        }
    }

    pub fn parent(&self) -> Option<WhPath> {
        // REVIEW - clones for now, as can't make a Whpath that has '&Utf8Path' as inner type
        Some(Self {
            inner: self.inner.parent()?.into(),
        })
    }
}

/* NOTE - removed as we don't normalize paths here, only check for it. But could still be useful

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
*/

pub fn osstring_to_string(osstr: OsString) -> WhResult<String> {
    osstr
        .into_string()
        .map_err(|_| WhPathError::NotValidUtf8.into())
}

pub fn osstr_to_str(osstr: &OsStr) -> WhResult<&str> {
    osstr.to_str().ok_or(WhPathError::NotValidUtf8.into())
}

/// Checks for:
/// - Path is NOT absolute
/// - Path is normalized
pub fn is_valid_for_whpath<T: AsRef<Path>>(p: T) -> Result<(), WhPathError> {
    let p = p.as_ref();

    if p.is_absolute() {
        return Err(WhPathError::NotRelative);
    }
    if p.components()
        .any(|c| c.as_os_str() == ".." || c.as_os_str() == ".")
    {
        return Err(WhPathError::NotNormalized);
    }
    Ok(())
}