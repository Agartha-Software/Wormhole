use crate::{
    functionnal::{
        append_to_path,
        environment_manager::{
            types::{StartupFiles, SLEEP_TIME},
            utilities::{assert_dirs_are_equal, tree_command},
        },
        start_log,
    },
    test_data,
};

use super::environment_manager;

pub use environment_manager::EnvironmentManager;
use serial_test::serial;

#[serial]
#[test]
fn base_files_before_mount() {
    start_log();
    log::info!("vvvvvv base_files_before_mount vvvvvv");
    let mut env = EnvironmentManager::new();
    env.add_service().unwrap();
    env.add_service().unwrap();
    env.add_service().unwrap();
    std::thread::sleep(*SLEEP_TIME);
    log::debug!("before network");
    env.create_network(
        "default".to_string(),
        Some(StartupFiles::VeryFirstOnly(
            test_data::SIMPLE_RECURSIVE.into(),
        )),
    )
    .unwrap();
    log::debug!("after network");

    std::thread::sleep(*SLEEP_TIME);

    for paths in [
        &env.services[0].pods[0].2.path().to_owned(),
        &env.services[1].pods[0].2.path().to_owned(),
        &env.services[2].pods[0].2.path().to_owned(),
    ] {
        log::debug!("assert dirs are equal for {}", paths.to_string_lossy());
        log::debug!("tree: {}", tree_command(&["-a", paths.to_str().unwrap()]));
        assert_dirs_are_equal(paths, test_data::SIMPLE_RECURSIVE);
    }
    std::thread::sleep(*SLEEP_TIME);
    log::info!("^^^^^^ base_files_before_mount ^^^^^^");
}
