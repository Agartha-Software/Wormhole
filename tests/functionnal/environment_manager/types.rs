use std::{env::var, path::PathBuf, time::Duration};

use assert_fs::TempDir;
// Takes the SLEEP_TIME env variable or default to 2sec
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SLEEP_TIME: Duration =
        Duration::from_secs_f32(if let Ok(str_st) = var("SLEEP_TIME") {
            str_st.parse().unwrap_or(2.0)
        } else {
            2.0
        });
}

pub const MAX_PORT: u16 = 40100;
pub const MIN_PORT: u16 = 40000;
pub const SERVICE_BIN: &str = "./target/debug/wormholed";
pub const CLI_BIN: &str = "./target/debug/wormhole";

pub struct Service {
    pub instance: std::process::Child,
    pub port: u16,
    pub pods: Vec<(String, u16, TempDir)>, // (network_name, ip, dir)
}

impl Drop for Service {
    fn drop(&mut self) {
        let exit_status = self.instance.wait();
        std::thread::sleep(*SLEEP_TIME);

        match &exit_status {
            Ok(status) => log::info!(
                "Stopped service {}\nExitStatus: {:?}\nStopped pods:\n{:?}",
                self.port,
                status,
                self.pods
                    .iter()
                    .map(|(_, ip, _)| ip.to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
            Err(e) => log::error!(
                "Error when stopping service {}\nExit error: {:?}\n^ This service pods:\n{:?}",
                self.port,
                e,
                self.pods
                    .iter()
                    .map(|(_, port, _)| port.to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            ),
        }
    }
}

pub enum StopMethod {
    CtrlD,
    CliStop,
    Kill,
}

/// Whether or not giving new pods files before mounting
pub enum StartupFiles {
    /// All pods gets a copy
    ForAll(PathBuf),
    /// Only the first pod of the network gets a copy
    VeryFirstOnly(PathBuf),
    /// The first pod of this batch gets a copy (even if older pods are already in the network)
    CurrentFirst(PathBuf),
}

impl From<StartupFiles> for PathBuf {
    fn from(startup_files: StartupFiles) -> Self {
        match startup_files {
            StartupFiles::ForAll(path)
            | StartupFiles::VeryFirstOnly(path)
            | StartupFiles::CurrentFirst(path) => path,
        }
    }
}
