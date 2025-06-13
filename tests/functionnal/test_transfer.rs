use crate::functionnal::append_to_path;

use super::environnement_manager;

pub use environnement_manager::EnvironnementManager;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn basic_text_file_transfer() {
    println!("====== STARTING BASIC TEXT FILE TRANSFER ========");
    let mut env = EnvironnementManager::new();
    env.add_service(false).unwrap();
    env.add_service(false).unwrap();
    env.add_service(false).unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));
    env.create_network("default".to_string(), false)
        .await
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));
    let file_path = append_to_path(&env.services[0].pods[0].2.path().to_owned(), "/foo.txt");
    std::fs::write(&file_path, "Hello world!").unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));

    for paths in [
        &env.services[0].pods[0].2.path().to_owned(),
        &env.services[1].pods[0].2.path().to_owned(),
        &env.services[2].pods[0].2.path().to_owned(),
    ] {
        match std::fs::read_to_string(append_to_path(paths, "/foo.txt")) {
            Ok(content) => assert!(content == "Hello world!", "File content is incorrect"),
            Err(_) => assert!(false, "File doesn't exist"),
        }
    }
    std::thread::sleep(std::time::Duration::from_secs_f32(1.));
}

//Does not work yet
/*
#[serial]
#[tokio::test]
async fn basic_nested_folder_transfer() {
    println!("Started: basic_nested_folder_transfer");

    let mut env = EnvironnementManager::new();
    env.add_service(true).unwrap();
    env.add_service(true).unwrap();
    env.add_service(true).unwrap();

    std::thread::sleep(std::time::Duration::from_secs_f32(0.5));
    std::fs::create_dir_all(append_to_path(&env.services[0].path, "/dir1/dir2")).unwrap();
    let file_path = append_to_path(&env.services[0].path, "/dir1/dir2/foo.txt");
    std::thread::sleep(std::time::Duration::from_secs_f32(0.3));
    std::fs::write(&file_path, "Hello world!").unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(0.3));

    for paths in [&env.services[1].path, &env.services[2].path] {
        match std::fs::metadata(append_to_path(paths, "/dir1")) {
            Ok(meta) => assert!(meta.is_dir(), "Dir1 isn't a directory"),
            Err(_) => assert!(false, "Dir1 doesn't exist"),
        };
        match std::fs::metadata(append_to_path(paths, "/dir1/dir2")) {
            Ok(meta) => assert!(meta.is_dir(), "Dir2 isn't a directory"),
            Err(_) => assert!(false, "Dir2 doesn't exist"),
        };
        match std::fs::read_to_string(append_to_path(paths, "/dir1/dir2/foo.txt")) {
            Ok(content) => assert!(content == "Hello world!", "File content is incorrect"),
            Err(_) => assert!(false, "File doesn't exist"),
        }
    }
    std::thread::sleep(std::time::Duration::from_secs_f32(1.));
}
*/
