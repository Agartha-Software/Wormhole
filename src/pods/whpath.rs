use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::str::FromStr;
use std::{fmt, path::Path};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum PathType {
    Absolute,
    Relative,
    NoPrefix,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct WhPath {
    pub inner: String,
    pub kind: PathType,
}

pub trait JoinPath {
    fn as_str(&self) -> &str;
}

impl JoinPath for OsStr {
    fn as_str(&self) -> &str {
        self.to_str().expect("OsStr conversion to str failed")
    }
}

impl JoinPath for &str {
    fn as_str(&self) -> &str {
        self
    }
}

impl JoinPath for str {
    fn as_str(&self) -> &str {
        self
    }
}

impl JoinPath for String {
    fn as_str(&self) -> &str {
        self.as_str()
    }
}

impl JoinPath for Path {
    fn as_str(&self) -> &str {
        self.to_str().expect("Path conversion to str failed")
    }
}

impl JoinPath for WhPath {
    fn as_str(&self) -> &str {
        &self.inner
    }
}

impl fmt::Display for PathType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let path_type: &str = match self {
            &PathType::Absolute => "Asbolute",
            &PathType::Relative => "Relative",
            &PathType::NoPrefix => "NoPrefix",
            &PathType::Empty => "Empty",
        };
        write!(f, "{}", path_type)
    }
}

impl fmt::Display for WhPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T> From<&T> for WhPath
where
    T: JoinPath + ?Sized,
{
    fn from(path: &T) -> Self {
        let mut wh_path = WhPath {
            inner: path.as_str().to_string(),
            kind: PathType::Empty,
        };
        wh_path.update_kind();
        wh_path
    }
}

impl Into<OsString> for &WhPath {
    fn into(self) -> OsString {
        OsString::from_str(&self.inner).expect("infaillable")
    }
}

impl WhPath {
    pub fn new() -> Self {
        WhPath {
            inner: String::from(""),
            kind: PathType::Empty,
        }
    }

    /// Add a segment to the current WhPath. If the segment starts with a `/` or `./` or `../`, the leading slash is removed.
    /// If the current WhPath is empty, the segment is added as is.
    /// If the current WhPath is not empty, the segment is added after adding a slash at the end of the current WhPath.
    /// # Examples
    ///
    pub fn push<T>(&mut self, segment: &T) -> &Self
    where
        T: JoinPath + ?Sized,
    {
        self.add_last_slash();
        let seg = Self::remove_leading_slash(segment.as_str());
        self.inner = format!("{}{}", self.inner, seg);
        self
    }

    /// Join the current path with a new segment. If the segment starts with a `/` or `./` or `../`, the leading slash is removed.
    /// If the current path is empty, the segment is added as is.
    /// If the current path is not empty, the segment is added after adding a slash at the end of the current path.
    /// If the segment is hidden (`is_hidden()` return true), the segment is added as is, without adding a slash.
    /// # Examples
    ///
    pub fn join<T>(&self, segment: &T) -> Self
    where
        T: JoinPath + ?Sized,
    {
        let mut pth = self.clone();

        pth.add_last_slash();

        let seg = Self::remove_leading_slash(segment.as_str());
        pth.inner = format!("{}{}", pth.inner, seg);
        pth
    }

    /// Remove the requested part
    /// “/my/file/path/”.remove(“file/path”) == “/my/”
    pub fn remove<T>(&mut self, delete_this_part: &T) -> &Self
    where
        T: JoinPath + ?Sized,
    {
        self.inner = self.inner.replace(delete_this_part.as_str(), "");
        self.delete_double_slash();
        self.convert_path(self.kind.clone());
        self
    }

    /// Modify the path to match the new name.
    /// Can only modify the last element of the path.
    pub fn rename<T>(&mut self, file_name: &T) -> &Self
    where
        T: JoinPath + ?Sized,
    {
        let file = Self::remove_leading_slash(file_name.as_str());
        if let Some(pos) = self.inner.rfind('/') {
            if pos == self.inner.len() - 1 {
                self.inner.pop();
                self.rename(file_name);
                self.inner.push('/');
                return self;
            }
            self.inner = format!("{}/{}", &self.inner[..pos], file);
        } else {
            self.inner = file.to_string();
        }
        self.update_kind();
        self
    }

    pub fn kind(&self) -> PathType {
        if self.is_empty() {
            return PathType::Empty;
        }
        if self.inner.chars().next() == Some('.') {
            return PathType::Relative;
        } else if self.inner.chars().next() == Some('/') {
            return PathType::Absolute;
        } else {
            return PathType::NoPrefix;
        }
    }

    pub fn update_kind(&mut self) {
        self.kind = self.kind();
    }

    /// Change the path for "./path"
    pub fn set_relative(mut self) -> Self {
        if !self.is_empty() && !Self::is_relative(&self) {
            self.convert_path(PathType::Relative);
        }
        self
    }

    /// Change the path for "/path"
    pub fn set_absolute(mut self) -> Self {
        if !self.is_empty() && !Self::is_absolute(&self) {
            self.convert_path(PathType::Absolute);
        }
        self
    }

    /// Change the path for "path"
    pub fn remove_prefix(mut self) -> Self {
        if !self.is_empty() && !Self::has_no_prefix(&self) {
            self.convert_path(PathType::NoPrefix);
        }
        self
    }

    pub fn is_relative(&self) -> bool {
        self.kind == PathType::Relative
    }

    pub fn is_absolute(&self) -> bool {
        self.kind == PathType::Absolute
    }

    pub fn has_no_prefix(&self) -> bool {
        self.kind == PathType::NoPrefix
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Put or not a '/' at the end
    pub fn set_end(&mut self, end: bool) -> &Self {
        if end {
            self.add_last_slash();
        } else {
            self.remove_last_slash();
        };
        self
    }

    /// Return true if the requested path is in the original path
    pub fn is_in<T>(&self, segment: &T) -> bool
    where
        T: JoinPath + ?Sized,
    {
        self.inner.starts_with(segment.as_str())
    }

    /// Give the last element of the path
    pub fn get_end(&self) -> String {
        let mut path = self.clone();
        path.remove_last_slash();
        match path.inner.rsplit('/').next() {
            Some(last) => last.to_string(),
            _none => String::new(),
        }
    }

    /// Returns all but the last element
    pub fn get_folder(&self) -> String {
        let mut path = self.clone();
        path.remove_last_slash();
        match path.inner.rsplit_once('/') {
            Some((first, _)) => first.to_string(),
            _none => String::new(),
        }
    }

    pub fn split_folder_file(&self) -> (String, String) {
        let mut path = self.clone();
        path.remove_last_slash();
        match path.inner.rsplit_once('/') {
            Some((first, last)) => (first.to_string(), last.to_string()),
            None => (String::new(), path.inner),
        }
    }

    pub fn pop(&mut self) -> &Self {
        self.remove_last_slash();
        if let Some(pos) = self.inner.rfind('/') {
            self.inner = self.inner[..(pos + 1)].to_string();
        } else {
            self.convert_path(PathType::Empty);
        }
        return self;
    }

    pub fn to_vector(&mut self) -> Vec<String> {
        let mut elements: Vec<String> = vec![];

        while !self.is_empty() {
            if !self.get_end().is_empty() {
                elements.push(self.get_end());
            }
            self.pop();
        }
        let elements = elements.into_iter().rev().collect();
        elements
    }

    //FIXME - Do I need to modify it for Windows by adding the '\'?
    //FIXME - Do I need to modify it to take hidden files into account?
    fn remove_leading_slash(segment: &str) -> &str {
        let mut j = 0;
        let mut i = 0;
        let chars: Vec<char> = segment.chars().collect();
        while i < chars.len() {
            if chars[i] == '.' {
                if i + 1 < chars.len() && (chars[i + 1] == '/' || chars[i + 1] == '.') {
                    i += 1;
                    j += 2;
                } else {
                    break;
                }
            } else if chars[i] == '/' {
                j += 1;
            } else {
                break;
            }
            i += 1;
        }
        return &segment[j..];
    }

    //FIXME - Should we modify it for Windows by adding the ‘\’?
    fn add_last_slash(&mut self) -> &Self {
        if self.kind != PathType::Empty && self.inner.chars().last() != Some('/') {
            self.inner = format!("{}/", self.inner);
        }
        return self;
    }

    //FIXME - Should we modify it for Windows by adding the ‘\’?
    fn remove_last_slash(&mut self) -> &Self {
        if let Some(pos) = self.inner.rfind('/') {
            if pos == self.inner.len() - 1 {
                self.inner.pop();
            }
        }
        return self;
    }

    fn delete_double_slash(&mut self) -> &Self {
        let mut i = 0;
        let mut index = 0;
        while index < self.inner.len() {
            i = if self.inner.as_bytes()[index] == b'/' {
                i + 1
            } else {
                0
            };
            if i == 2 {
                self.inner.remove(index);
                i -= 1;
                continue;
            }

            index += 1;
        }
        return self;
    }

    //FIXME - Should we modify it for Windows by adding the ‘\’?
    fn convert_path(&mut self, pathtype: PathType) -> &Self {
        if pathtype == PathType::Empty || self.inner == String::new() {
            self.inner = String::new();
            self.kind = PathType::Empty;
            return self;
        }
        self.inner = Self::remove_leading_slash(&self.inner.clone()).to_string();
        if pathtype == PathType::Absolute {
            self.inner = format!("/{}", self.inner);
        }
        if pathtype == PathType::Relative {
            self.inner = format!("./{}", self.inner);
        } else {
        }
        self.update_kind();
        return self;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_whpath_remove_leading_slash() {
        assert_eq!(WhPath::remove_leading_slash("bar"), "bar");
        assert_eq!(WhPath::remove_leading_slash("./bar"), "bar");
        assert_eq!(WhPath::remove_leading_slash("/bar"), "bar");
        assert_eq!(WhPath::remove_leading_slash(""), "");
        assert_eq!(WhPath::remove_leading_slash(".bar"), ".bar");
    }

    #[test]
    fn test_whpath_add_last_slash() {
        let mut empty = WhPath::new();
        let mut basic_folder = WhPath::from("foo/");
        let mut no_slash = WhPath::from("baz");

        assert_eq!(empty.add_last_slash(), &WhPath::new());
        assert_eq!(basic_folder.add_last_slash(), &WhPath::from("foo/"));
        assert_eq!(no_slash.add_last_slash(), &WhPath::from("baz/"));
    }

    #[test]
    fn test_whpath_remove_last_slash() {
        let mut empty = WhPath::new();
        let mut basic_folder = WhPath::from("foo/");
        let mut no_slash = WhPath::from("baz/bar");

        assert_eq!(empty.remove_last_slash(), &WhPath::new());
        assert_eq!(basic_folder.remove_last_slash(), &WhPath::from("foo"));
        assert_eq!(no_slash.remove_last_slash(), &WhPath::from("baz/bar"));
    }

    #[test]
    fn test_whpath_delete_double_slash() {
        let mut empty = WhPath::new();
        let mut basic_folder = WhPath::from("foo/");
        let mut mid_double_slash = WhPath::from("baz//bar");
        let mut end_double_slash = WhPath::from("baz/bar//");

        assert_eq!(empty.delete_double_slash(), &WhPath::new());
        assert_eq!(basic_folder.delete_double_slash(), &WhPath::from("foo/"));
        assert_eq!(
            mid_double_slash.delete_double_slash(),
            &WhPath::from("baz/bar")
        );
        assert_eq!(
            end_double_slash.delete_double_slash(),
            &WhPath::from("baz/bar/")
        );
    }

    #[test]
    fn test_whpath_convert_path() {
        let mut path = WhPath::from("foo");
        let mut relative = WhPath::from("./foo");
        let mut no_prefix = WhPath::from("foo");
        let mut absolute = WhPath::from("/foo");
        let mut empty = WhPath::from("");

        assert_eq!(relative.convert_path(PathType::NoPrefix), &no_prefix);
        assert_eq!(no_prefix.convert_path(PathType::Absolute), &absolute);
        assert_eq!(absolute.convert_path(PathType::Empty), &empty);
        assert_eq!(empty.convert_path(PathType::Absolute), &WhPath::new());
        assert_eq!(
            path.convert_path(PathType::Relative),
            &WhPath::from("./foo")
        );
    }

    #[test]
    fn test_whpath_to_vector() {
        let mut path = WhPath::from("foo/pouet/lol");

        assert_eq!(path.to_vector(), vec!["foo", "pouet", "lol"]);
    }
}
