extern crate wormhole;
use std::path::PathBuf;

use serial_test::parallel;
use wormhole::pods::whpath::normalize_path;

#[parallel]
#[test]
fn test_path_normalization() {
  let windows_root_path = PathBuf::from(r"C:\windows\system32.dll");
  let windows_root_path_dot = PathBuf::from(r"C:\windows\.\system32.dll");
  let windows_verbatim_root_path = PathBuf::from(r"\\?\C:\windows\system32.dll");
  let windows_verbatim_root_path_dot = PathBuf::from(r"\\?\C:\windows\.\system32.dll");
  let windows_relative_path = PathBuf::from(r".\relative\path");
  let windows_relative_path_no_prefix = PathBuf::from(r".\relative\path");
  let windows_relative_path_one_up = PathBuf::from(r".\relative\path\..");
  let windows_relative_path_one_up_one_down = PathBuf::from(r".\relative\path\..\down");
  let windows_relative_impossible_up = PathBuf::from(r".\relative\path\..\..\..");
  let windows_relative_impossible_up2 = PathBuf::from(r".\relative\path\..\..\..\..");
  let windows_root_impossible_up = PathBuf::from(r"C:\relative\path\..\..\..");
  let windows_root_impossible_up2 = PathBuf::from(r"C:\relative\path\..\..\..\..");
  let windows_verbatim_root_impossible_up = PathBuf::from(r"\\?\C:\relative\path\..\..\..");
  let windows_verbatim_root_impossible_up2 = PathBuf::from(r"\\?\C:\relative\path\..\..\..\..");
  let windows_root_that_goes_all_up = PathBuf::from(r"C:\relative\path\..\..");
  let windows_relative_that_goes_all_up = PathBuf::from(r".\relative\path\..\..");

  let linux_root_path = PathBuf::from("/linux/system32.dll");
  let linux_root_path_dot = PathBuf::from("/linux/./system32.dll");
  let linux_relative_path = PathBuf::from("./relative/path");
  let linux_relative_path_no_prefix = PathBuf::from("relative/path");
  let linux_relative_path_dot = PathBuf::from("./relative/./path");
  let linux_relative_path_one_up = PathBuf::from("./relative/path/..");
  let linux_relative_path_one_up_one_down = PathBuf::from("./relative/path/../down");
  let linux_relative_impossible_up = PathBuf::from("./relative/path/../../..");
  let linux_relative_impossible_up2 = PathBuf::from("./relative/path/../../../..");
  let linux_root_impossible_up = PathBuf::from("/relative/path/../../..");
  let linux_root_impossible_up2 = PathBuf::from("/relative/path/../../../..");
  let linux_root_that_goes_all_up = PathBuf::from("/relative/path/../..");
  let linux_relative_that_goes_all_up = PathBuf::from("./relative/path/../..");

  // Windows absolute roots
  let r = normalize_path(&windows_root_path);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"C:\windows\system32.dll"));

  let r = normalize_path(&windows_root_path_dot);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"C:\windows\system32.dll"));

  let r = normalize_path(&windows_verbatim_root_path);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"\\?\C:\windows\system32.dll"));

  let r = normalize_path(&windows_verbatim_root_path_dot);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"\\?\C:\windows\system32.dll"));

  let r = normalize_path(&windows_root_that_goes_all_up);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"C:\"));

  // Windows relative paths
  let r = normalize_path(&windows_relative_path);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"relative\path"));

  let r = normalize_path(&windows_relative_path_no_prefix);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"relative\path"));

  let r = normalize_path(&windows_relative_path_one_up);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"relative"));

  let r = normalize_path(&windows_relative_path_one_up_one_down);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from(r"relative\down"));

  let r = normalize_path(&windows_relative_that_goes_all_up);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::default());

  // Windows relative paths that go up too far
  assert!(normalize_path(&windows_relative_impossible_up).is_err());
  assert!(normalize_path(&windows_relative_impossible_up2).is_err());

  // Windows absolute paths that try to go above root
  assert!(normalize_path(&windows_root_impossible_up).is_err());
  assert!(normalize_path(&windows_root_impossible_up2).is_err());

  // Verbatim Windows absolute paths that try to go above root
  assert!(normalize_path(&windows_verbatim_root_impossible_up).is_err());
  assert!(normalize_path(&windows_verbatim_root_impossible_up2).is_err());

  // Linux absolute roots
  let r = normalize_path(&linux_root_path);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("/linux/system32.dll"));

  let r = normalize_path(&linux_root_path_dot);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("/linux/system32.dll"));

  let r = normalize_path(&linux_root_that_goes_all_up);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("/"));

  // Linux relative paths
  let r = normalize_path(&linux_relative_path);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("relative/path"));

  let r = normalize_path(&linux_relative_path_no_prefix);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("relative/path"));

  let r = normalize_path(&linux_relative_that_goes_all_up);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::default());

  let r = normalize_path(&linux_relative_path_dot);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("relative/path"));

  let r = normalize_path(&linux_relative_path_one_up);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("relative"));

  let r = normalize_path(&linux_relative_path_one_up_one_down);
  assert!(r.is_ok());
  assert_eq!(r.unwrap(), PathBuf::from("relative/down"));

  // Linux relative paths that go up too far -> error
  assert!(normalize_path(&linux_relative_impossible_up).is_err());
  assert!(normalize_path(&linux_relative_impossible_up2).is_err());

  // Linux absolute paths that try to go above root -> error
  assert!(normalize_path(&linux_root_impossible_up).is_err());
  assert!(normalize_path(&linux_root_impossible_up2).is_err());
}
