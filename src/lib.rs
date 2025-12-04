// In rust we code
// In code we trust
// AgarthaSoftware - 2024

//! Wormhole
//!
//! Checkout the [CLI](../wormhole_cli/index.html)
//!
//! Checkout the [Service](../wormhole_service/index.html)
//!

pub mod cli;
pub mod config;
pub mod data;
pub mod error;
pub mod ipc;
pub mod network;
pub mod pods;
pub mod signals;
pub mod startup;
#[cfg(target_os = "windows")]
pub mod winfsp;

#[cfg(target_os = "windows")]
pub const INSTANCE_PATH: &str = "%APPDATA%/local/wormhole";

#[cfg(target_os = "linux")]
pub const INSTANCE_PATH: &'static str = "/usr/local/share/wormhole/";
#[cfg(target_os = "linux")]
pub mod fuse;
