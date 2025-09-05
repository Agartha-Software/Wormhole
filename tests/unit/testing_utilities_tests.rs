use serial_test::parallel;

use crate::{
    functionnal::environment_manager::utilities::{assert_dirs_are_equal, copy_dir_all},
    test_data::{SIMPLE, SIMPLE_RECURSIVE},
};

#[parallel]
#[test]
fn equal_dirs() {
    assert_dirs_are_equal(SIMPLE_RECURSIVE, SIMPLE_RECURSIVE);
}

#[parallel]
#[test]
#[should_panic]
fn non_equal_dirs() {
    assert_dirs_are_equal(SIMPLE_RECURSIVE, SIMPLE);
}

#[parallel]
#[test]
fn copy_dir_simple() {
    let temp_dir = assert_fs::TempDir::new().expect("can't create temp dir");
    assert!(copy_dir_all(SIMPLE, temp_dir.path()).is_ok());
    assert_dirs_are_equal(SIMPLE, temp_dir.path());
}

#[parallel]
#[test]
fn copy_dir_recursive() {
    let temp_dir = assert_fs::TempDir::new().expect("can't create temp dir");
    assert!(copy_dir_all(SIMPLE_RECURSIVE, temp_dir.path()).is_ok());
    assert_dirs_are_equal(SIMPLE_RECURSIVE, temp_dir.path());
}
