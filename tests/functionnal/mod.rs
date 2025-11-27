pub mod environment_manager;
pub mod test_base_files;
pub mod test_sync;
//pub mod test_transfer; // waiting for a fix in redundancy

pub use environment_manager::EnvironmentManager;

fn start_log() {
    let _ = env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .format_indent(Some(8))
        .is_test(true)
        .try_init();
}
