use crate::functionnal::{append_to_path, environment_manager::types::SLEEP_TIME, start_log};

use super::environment_manager;

pub use environment_manager::EnvironmentManager;
use serial_test::serial;

#[serial]
#[test]
fn sync_start_state() {
    start_log();
    log::info!("vvvvvv sync start state vvvvvv");
    let mut env = EnvironmentManager::new();
    env.add_service().unwrap();
    std::thread::sleep(*SLEEP_TIME);
    env.create_network("default".to_owned(), None).unwrap();
    std::thread::sleep(*SLEEP_TIME);
    let file_path = append_to_path(&env.services[0].pods[0].2.path().to_owned(), "/foo.txt");
    std::fs::write(&file_path, "Hello world!").unwrap();
    std::thread::sleep(*SLEEP_TIME);

    env.add_service().unwrap();
    std::thread::sleep(*SLEEP_TIME);
    env.create_network("default".to_owned(), None).unwrap();
    std::thread::sleep(*SLEEP_TIME);

    let check_path = append_to_path(&env.services[1].pods[0].2.path().to_owned(), "/foo.txt");
    match std::fs::read_to_string(&check_path) {
        Err(_) => assert!(false, "File doesn't exist"),
        Ok(content) => assert!(
            content == "Hello world!",   /*content == "Hello world!"*/
            "File content is incorrect"  // No support for file streaming yet
        ),
    }

    let file_path = append_to_path(&env.services[0].pods[0].2.path().to_owned(), "/bar.txt");
    std::fs::write(&file_path, "Goodbye world!").unwrap();
    env.add_service().unwrap();
    std::thread::sleep(*SLEEP_TIME);
    env.create_network("default".to_owned(), None).unwrap();
    std::thread::sleep(*SLEEP_TIME);

    for path in [
        &env.services[0].pods[0].2.path().to_owned(),
        &env.services[1].pods[0].2.path().to_owned(),
        &env.services[2].pods[0].2.path().to_owned(),
    ] {
        let path = append_to_path(path, "/bar.txt");
        match std::fs::read_to_string(&path) {
            Err(_) => assert!(false, "File {:?} doesn't exist", path),
            Ok(content) => assert!(
                content == "Goodbye world!", /*content == "Goodbye world!"*/
                "File content is incorrect"  // No support for file streaming yet
            ),
        }
    }
    std::thread::sleep(*SLEEP_TIME);
    log::info!("^^^^^^ sync start state ^^^^^^");
}
