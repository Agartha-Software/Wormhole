#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file;

use crate::functionnal::{environment_manager::types::SLEEP_TIME, start_log};

use super::environment_manager;

pub use environment_manager::EnvironmentManager;
use serial_test::serial;

#[serial]
#[test]
fn test_symlink() {
    start_log();
    log::info!("vvvvvv basic_text_file_transfer vvvvvv");
    let mut env = EnvironmentManager::new();
    env.add_service().unwrap();
    // env.add_service().unwrap();
    std::thread::sleep(*SLEEP_TIME);
    env.create_network("default".to_string(), None).unwrap();

    std::thread::sleep(*SLEEP_TIME);
    let file_path = &env.services[0].pods[0].2.path().to_owned().join("foo.txt");
    std::fs::write(&file_path, "Hello world!").unwrap();
    std::thread::sleep(*SLEEP_TIME);

    let link_path = &env.services[0].pods[0].2.path().join("link");
    #[cfg(target_os = "windows")]
    symlink_file(file_path, link_path).expect("creating symlink");
    assert_eq!(
        &std::fs::read_to_string(&link_path).expect("read link file"),
        "Hello world!"
    );

    // for paths in [
    //     &env.services[0].pods[0].2.path().to_owned(),
    //     &env.services[1].pods[0].2.path().to_owned(),
    // ] {
    //     match std::fs::read_to_string(append_to_path(paths, "/foo.txt")) {
    //         Ok(content) => assert!(content == "Hello world!", "File content is incorrect"),
    //         Err(_) => assert!(false, "File doesn't exist"),
    //     }
    // }
    std::thread::sleep(*SLEEP_TIME);
    log::info!("^^^^^^ basic_text_file_transfer ^^^^^^");
}
