extern crate wormhole;
use serial_test::parallel;

use crate::wormhole::pods::whpath::{JoinPath, PathType, WhPath};

use std::ffi::OsStr;
use std::path::Path;

#[parallel]
#[test]
fn test_whpath_kind() {
    let empty = WhPath {
        inner: String::from(""),
        kind: PathType::Empty,
    };
    let absolute = WhPath {
        inner: String::from("/foo"),
        kind: PathType::Empty,
    };
    let relative = WhPath {
        inner: String::from("./foo"),
        kind: PathType::Empty,
    };
    let noprefix = WhPath {
        inner: String::from("foo"),
        kind: PathType::Empty,
    };

    assert_eq!(empty.kind(), PathType::Empty);
    assert_eq!(absolute.kind(), PathType::Absolute);
    assert_eq!(relative.kind(), PathType::Relative);
    assert_eq!(noprefix.kind(), PathType::NoPrefix);
}

#[parallel]
#[test]
fn test_whpath_new() {
    let path = WhPath::from(OsStr::new("./StarWars/A_New_Hope"));
    assert_eq!(
        path,
        WhPath {
            inner: String::from("./StarWars/A_New_Hope"),
            kind: PathType::Relative
        }
    );

    let path = WhPath::from(Path::new("/HarryPotter/The_Goblet_of_Fire"));
    assert_eq!(
        path,
        WhPath {
            inner: String::from("/HarryPotter/The_Goblet_of_Fire"),
            kind: PathType::Absolute
        }
    );

    let path = WhPath::from("TheLordofTheRings/The_Two_Towers/");
    assert_eq!(
        path,
        WhPath {
            inner: String::from("TheLordofTheRings/The_Two_Towers/"),
            kind: PathType::NoPrefix
        }
    );

    let path = WhPath::new();
    assert_eq!(
        path,
        WhPath {
            inner: String::new(),
            kind: PathType::Empty
        }
    );
}

fn whpath_join<T: JoinPath>(path: WhPath, join: &T, result: &WhPath)
where
    T: JoinPath + ?Sized,
{
    let new_path = path.join(join);
    assert_eq!(&new_path, result);
}

#[parallel]
#[test]
fn test_whpath_join_with_str() {
    let path1 = WhPath::from("Kaamelott");
    let path2 = WhPath::from("./Kaamelott/");
    let path3 = WhPath::from("/Season 2/");

    whpath_join(
        path1.clone(),
        "./Season 1/",
        &WhPath::from("Kaamelott/Season 1/"),
    );
    whpath_join(path1.clone(), &path3, &WhPath::from("Kaamelott/Season 2/"));
    whpath_join(
        path1.clone(),
        OsStr::new("Season 3/"),
        &WhPath::from("Kaamelott/Season 3/"),
    );
    whpath_join(
        path1.clone(),
        Path::new("Season 4"),
        &WhPath::from("Kaamelott/Season 4"),
    );

    whpath_join(
        path1.clone(),
        &String::from("Season 5"),
        &WhPath::from("Kaamelott/Season 5"),
    );

    whpath_join(path1.clone(), "", &WhPath::from("Kaamelott/"));

    whpath_join(
        path2.clone(),
        "./Season 1/",
        &WhPath::from("./Kaamelott/Season 1/"),
    );
    whpath_join(
        path2.clone(),
        &path3,
        &WhPath::from("./Kaamelott/Season 2/"),
    );
    whpath_join(
        path2.clone(),
        &String::from("Season 3/"),
        &WhPath::from("./Kaamelott/Season 3/"),
    );
    whpath_join(
        path2.clone(),
        OsStr::new("Season 4"),
        &WhPath::from("./Kaamelott/Season 4"),
    );

    whpath_join(
        path2.clone(),
        Path::new("Season 5"),
        &WhPath::from("./Kaamelott/Season 5"),
    );

    whpath_join(path2.clone(), "", &WhPath::from("./Kaamelott/"));
}

#[parallel]
#[test]
fn test_whpath_remove() {
    let mut delete_all = WhPath::from("Film/Orson Welles/Citizen Kane");
    let mut delete_end = WhPath::from("Film/Orson Welles/The Magnificent Ambersons");
    let mut delete_middle = WhPath::from("Film/Orson Welles/The Lady from Shanghai");
    let mut delete_start = WhPath::from("Film/Orson Welles/Touch of Evil");
    let mut error = WhPath::from("Film/Orson Welles/Chimes at Midnight");

    assert_eq!(delete_all.remove(&delete_all.clone()), &WhPath::new());
    assert_eq!(
        delete_end.remove(OsStr::new("The Magnificent Ambersons")),
        &WhPath::from("Film/Orson Welles/")
    );

    assert_eq!(
        delete_middle.remove(Path::new("Orson Welles")),
        &WhPath::from("Film/The Lady from Shanghai")
    );
    assert_eq!(
        delete_start.remove(&String::from("Film")),
        &WhPath::from("Orson Welles/Touch of Evil")
    );

    assert_eq!(
        error.remove("Series"),
        &WhPath::from("Film/Orson Welles/Chimes at Midnight")
    );
}

#[parallel]
#[test]
fn test_whpath_rename() {
    let mut empty = WhPath::new();
    let mut basic_folder = WhPath::from("./foo/");
    let mut basic_file = WhPath::from("foo/file.txt");
    let mut basic_subfolder = WhPath::from("foo/bar/");
    let mut path = WhPath::from("path/exemple");
    let modify_path = WhPath::from("/modifier");

    assert_eq!(empty.rename(OsStr::new("baz")), &WhPath::from("baz"));
    assert_eq!(
        basic_folder.rename(Path::new("bar")),
        &WhPath::from("./bar/")
    );
    assert_eq!(
        basic_file.rename(&String::from("foo.rs")),
        &WhPath::from("foo/foo.rs")
    );
    assert_eq!(basic_subfolder.rename("sub"), &WhPath::from("foo/sub/"));
    assert_eq!(path.rename(&modify_path), &WhPath::from("path/modifier"));
}

#[parallel]
#[test]
fn test_whpath_set_relative() {
    let no_prefix = WhPath::from("foo");
    let absolute = WhPath::from("/foo/");
    let empty = WhPath::new();

    assert_eq!(no_prefix.set_relative(), WhPath::from("./foo"));
    assert_eq!(absolute.set_relative(), WhPath::from("./foo/"));
    assert_eq!(empty.set_relative(), WhPath::new());
}

#[parallel]
#[test]
fn test_whpath_set_absolute() {
    let no_prefix = WhPath::from("foo");
    let relative = WhPath::from("./foo/");
    let empty = WhPath::new();

    assert_eq!(no_prefix.set_absolute(), WhPath::from("/foo"));
    assert_eq!(relative.set_absolute(), WhPath::from("/foo/"));
    assert_eq!(empty.set_absolute(), WhPath::new());
}

#[parallel]
#[test]
fn test_whpath_remove_prefix() {
    let absolute = WhPath::from("/foo");
    let relative = WhPath::from("./foo/");
    let empty = WhPath::new();

    assert_eq!(absolute.remove_prefix(), WhPath::from("foo"));
    assert_eq!(relative.remove_prefix(), WhPath::from("foo/"));
    assert_eq!(empty.remove_prefix(), WhPath::new());
}

#[parallel]
#[test]
fn test_whpath_is_relative() {
    let relative = WhPath::from("./foo");
    let not_relative = WhPath::from("foo");

    assert_eq!(relative.is_relative(), true);
    assert_eq!(not_relative.is_relative(), false);
}

#[parallel]
#[test]
fn test_whpath_is_absolute() {
    let absolute = WhPath::from("/foo");
    let not_absolute = WhPath::from("foo");

    assert_eq!(absolute.is_absolute(), true);
    assert_eq!(not_absolute.is_absolute(), false);
}
#[parallel]
#[test]
fn test_whpath_has_no_prefix() {
    let no_prefix = WhPath::from("foo");
    let not_no_prefix = WhPath::from("/foo");

    assert_eq!(no_prefix.has_no_prefix(), true);
    assert_eq!(not_no_prefix.has_no_prefix(), false);
}
#[parallel]
#[test]
fn test_whpath_is_empty() {
    let empty = WhPath::new();
    let not_empty = WhPath::from("foo");

    assert_eq!(empty.is_empty(), true);
    assert_eq!(not_empty.is_empty(), false);
}

#[parallel]
#[test]
fn test_whpath_set_end() {
    let mut set_end_true = WhPath::from("/foo");
    let mut set_end_false = WhPath::from("/foo/");
    assert_eq!(set_end_true.set_end(true), &WhPath::from("/foo/"));
    assert_eq!(set_end_false.set_end(false), &WhPath::from("/foo"));
}

#[parallel]
#[test]
fn test_whpath_is_in() {
    let path = WhPath::from("/foo/bar/baz.txt");
    let empty = WhPath::new();
    let path_into_path = WhPath::from("/foo/bar/");

    assert_eq!(path.is_in(OsStr::new("/foo/bar")), true);
    assert_eq!(path.is_in(Path::new("bar")), false);
    assert_eq!(path.is_in(&String::from("peanuts")), false);
    assert_eq!(path.is_in(&path_into_path), true);
    assert_eq!(empty.is_in("peanuts"), false);
    assert_eq!(empty.is_in(""), true);
}

#[parallel]
#[test]
fn test_whpath_get_end() {
    let empty = WhPath::new();
    let basic_folder = WhPath::from("foo/");
    let basic_file = WhPath::from("foo/file.txt");
    let basic_subfolder = WhPath::from("foo/bar/");
    let no_slash = WhPath::from("baz");

    assert_eq!(empty.get_end(), String::new());
    assert_eq!(basic_folder.get_end(), String::from("foo"));
    assert_eq!(basic_file.get_end(), String::from("file.txt"));
    assert_eq!(basic_subfolder.get_end(), String::from("bar"));
    assert_eq!(no_slash.get_end(), String::from("baz"));
}

#[parallel]
#[test]
fn test_whpath_pop() {
    let mut empty = WhPath::new();
    let mut basic_folder = WhPath::from("foo/");
    let mut basic_file = WhPath::from("foo/file.txt");
    let mut basic_subfolder = WhPath::from("foo/bar/");
    let mut no_slash = WhPath::from("baz");

    assert_eq!(empty.pop(), &WhPath::new());
    assert_eq!(basic_folder.pop(), &WhPath::new());
    assert_eq!(basic_file.pop(), &WhPath::from("foo/"));
    assert_eq!(basic_subfolder.pop(), &WhPath::from("foo/"));
    assert_eq!(no_slash.pop(), &WhPath::new());
}
