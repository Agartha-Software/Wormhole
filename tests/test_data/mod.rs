use std::{fmt::Display, path::Path};

use serial_test::parallel;

pub const SIMPLE: &str = "tests/test_data/simple";
pub const SIMPLE_RECURSIVE: &str = "tests/test_data/simple_recursive";

fn check_dir_exists(path: impl AsRef<Path> + Display) {
    assert!(std::fs::read_dir(&path).is_ok(), "{} does not exist.", path);
}

/// Checks that the test_data folders can be found
#[parallel]
#[test]
fn check_test_data() {
    check_dir_exists(SIMPLE);
    check_dir_exists(SIMPLE_RECURSIVE);
}
