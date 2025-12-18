use camino::{FromPathBufError, Iter, Utf8Path, Utf8PathBuf};
use custom_error::custom_error;
#[cfg(target_os = "linux")]
use openat::AsPath;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::ffi::CString;
use std::fmt::{Debug, Display};
use std::io;
use std::ops::Deref;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

custom_error! {pub WhPathError
    NotRelative = "Path is not relative",
    ConversionError{source: ConversionError} = "{source}",
    NotNormalized = "Path is not normal",
    InvalidOperation = "Operation would compromise WhPath integrity",
}

custom_error! { pub InodeNameError{} = "Name contains forbidden character(s)" }
custom_error! { pub ConversionError{} = "Could not be converted to valid UTF-8" }

impl ConversionError {
    pub fn to_libc(&self) -> i32 {
        libc::EILSEQ
    }

    pub fn into_io(self) -> io::Error {
        io::ErrorKind::InvalidData.into()
    }
}

impl InodeNameError {
    pub fn to_io(self) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidFilename, self.to_string())
    }

    pub fn to_libc(self) -> i32 {
        libc::EACCES
    }
}

impl From<FromPathBufError> for WhPathError {
    fn from(_: FromPathBufError) -> Self {
        Self::ConversionError {
            source: ConversionError {},
        }
    }
}

impl WhPathError {
    pub fn to_io(&self) -> io::Error {
        match self {
            WhPathError::NotRelative => io::Error::new(io::ErrorKind::Other, self.to_string()),
            WhPathError::ConversionError { source } => {
                io::Error::new(io::ErrorKind::InvalidData, source.to_string())
            }
            WhPathError::NotNormalized => {
                io::Error::new(io::ErrorKind::InvalidFilename, self.to_string())
            }
            WhPathError::InvalidOperation => io::Error::new(io::ErrorKind::Other, self.to_string()),
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
        is_valid_for_whpath(&p)?;

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

impl From<&InodeName> for WhPath {
    /// From InodeName is UNCHECKED as InodeNames names should already be correct
    fn from(value: &InodeName) -> Self {
        let p = Utf8PathBuf::from(value);

        Self { inner: p }
    }
}

#[cfg(target_os = "windows")]
impl TryFrom<&winfsp::U16CStr> for WhPath {
    type Error = WhPathError;

    fn try_from(value: &winfsp::U16CStr) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string().map_err(|_| ConversionError {})?)
    }
}

impl Into<String> for WhPath {
    fn into(self) -> String {
        self.inner.into_string()
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
            .map_err(|_| ConversionError {})?
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

pub fn osstring_to_string(osstr: OsString) -> Result<String, ConversionError> {
    osstr.into_string().map_err(|_| ConversionError {})
}

pub fn osstr_to_str(osstr: &OsStr) -> Result<&str, ConversionError> {
    osstr.to_str().ok_or(ConversionError {})
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

// SECTION Name

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct InodeName(String);

impl InodeName {
    pub fn check(name: &str) -> Result<(), InodeNameError> {
        let patterns = ["\\", "/"];
        match patterns.into_iter().any(|pat| name.contains(pat)) {
            true => Err(InodeNameError {}),
            false => Ok(()),
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for InodeName {
    type Error = InodeNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        InodeName::check(&value)?;
        Ok(Self(value))
    }
}

#[cfg(target_os = "windows")]
impl TryFrom<&winfsp::U16CStr> for InodeName {
    type Error = InodeNameError;

    fn try_from(value: &winfsp::U16CStr) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string().map_err(|_| InodeNameError {})?)
    }
}

impl TryFrom<OsString> for InodeName {
    type Error = InodeNameError;

    fn try_from(p: OsString) -> Result<Self, Self::Error> {
        Self::try_from(osstring_to_string(p).map_err(|_| InodeNameError {})?)
    }
}

impl Into<String> for InodeName {
    fn into(self) -> String {
        self.0
    }
}

impl<T> AsRef<T> for InodeName
where
    T: ?Sized,
    String: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

impl<T> PartialEq<T> for InodeName
where
    T: ?Sized + std::cmp::PartialEq,
    String: AsRef<T>,
{
    fn eq(&self, other: &T) -> bool {
        other == self.0.as_ref()
    }
}

impl From<&WhPath> for InodeName {
    fn from(value: &WhPath) -> Self {
        Self((*value).file_name().unwrap().to_owned()) // NOTE - unwrap allowed as only fails if path end on "..", which can't be the case on a WhPath
    }
}
