use std::{io::Write, path::PathBuf};

use serial_test::serial;

use crate::{
    functionnal::{
        environment_manager::types::{StartupFiles, SLEEP_TIME},
        start_log, EnvironmentManager,
    },
    test_data,
};

fn log_meta(path: PathBuf) {
    let metadata = std::fs::metadata(path).unwrap();
    log::debug!(
        "accessed: {:?}, modified: {:?}",
        metadata.accessed().unwrap(),
        metadata.modified().unwrap()
    );
}

#[serial]
#[test]
fn test_metabasic() {
    start_log();
    println!("Captured?");
    log::info!("vvvvvv basic metadata vvvvvv");

    let mut env = EnvironmentManager::new();
    env.add_service().unwrap();
    env.add_service().unwrap();

    std::thread::sleep(*SLEEP_TIME);
    log::debug!("before network");
    env.create_network(
        "default".to_string(),
        Some(StartupFiles::ForAll(test_data::SIMPLE.into())),
    )
    .unwrap();
    log::debug!("after network");

    std::thread::sleep(*SLEEP_TIME);

    for dir_path in [
        &env.services[0].pods[0].2.path().to_owned(),
        &env.services[1].pods[0].2.path().to_owned(),
    ] {
        let dir = std::fs::read_dir(dir_path).unwrap();

        for dir_entry in dir {
            let entry = dir_entry.unwrap();
            if !entry.file_name().to_string_lossy().starts_with('.') {
                log::debug!("Testing file: {}", entry.path().display());
                log_meta(entry.path());
                let content = std::fs::read_to_string(entry.path()).unwrap();
                log::debug!("content: {}", content);
                log_meta(entry.path());
                {
                    let mut file = std::fs::File::options()
                        .write(true)
                        .open(entry.path())
                        .unwrap();
                    file.write("test".as_bytes()).unwrap();
                }
                log_meta(entry.path());
            }
        }
    }
    std::thread::sleep(*SLEEP_TIME);
    log::info!("^^^^^^ base_files_before_mount ^^^^^^");
}
