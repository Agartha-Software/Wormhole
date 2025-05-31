use crate::functionnal::append_to_path;

use super::environnement_manager;

pub use environnement_manager::EnvironnementManager;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn sync_start_state() {
    println!("====== STARTING SYNC START STATE========");
    let mut env = EnvironnementManager::new();
    env.add_service(true).unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));
    env.create_network("default".to_owned(), true)
        .await
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));

    let file_path = append_to_path(&env.services[0].pods[0].2.path().to_owned(), "/foo.txt");
    std::fs::write(&file_path, "Hello world!").unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(1.0));

    env.add_service(false).unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));
    env.create_network("default".to_owned(), false)
        .await
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));

    let check_path = append_to_path(&env.services[1].pods[0].2.path().to_owned(), "/foo.txt");
    match std::fs::read_to_string(&check_path) {
        Err(_) => assert!(false, "File doesn't exist"),
        Ok(_content) => assert!(
            true,                        /*content == "Hello world!"*/
            "File content is incorrect"  // No support for file streaming yet
        ),
    }

    let file_path = append_to_path(&env.services[0].pods[0].2.path().to_owned(), "/bar.txt");
    std::fs::write(&file_path, "Goodbye world!").unwrap();
    env.add_service(false).unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));
    env.create_network("default".to_owned(), false)
        .await
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(2.0));

    for path in [
        &env.services[0].pods[0].2.path().to_owned(),
        &env.services[1].pods[0].2.path().to_owned(),
        &env.services[2].pods[0].2.path().to_owned(),
    ] {
        let path = append_to_path(path, "/bar.txt");
        match std::fs::read_to_string(&path) {
            Err(_) => assert!(false, "File {:?} doesn't exist", path),
            Ok(_content) => assert!(
                true,                        /*content == "Goodbye world!"*/
                "File content is incorrect"  // No support for file streaming yet
            ),
        }
    }
}
