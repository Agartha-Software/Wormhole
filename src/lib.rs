// In rust we code
// In code we trust
// AgarthaSoftware - 2024

//! Wormhole
//!
//! Checkout the [CLI](../wormhole_cli/index.html)
//!
//! Checkout the [Service](../wormhole_service/index.html)
//!

use std::ffi::OsStr;

pub mod commands;
pub mod config;
pub mod data;
pub mod error;
pub mod network;
pub mod pods;
#[cfg(target_os = "windows")]
pub mod winfsp;

#[cfg(target_os = "windows")]
pub const INSTANCE_PATH: &str = "%APPDATA%/local/wormhole";

#[cfg(target_os = "linux")]
pub const INSTANCE_PATH: &'static str = "/usr/local/share/wormhole/";
#[cfg(target_os = "linux")]
pub mod fuse;

/// This function was created to reduce the boiler plate of string conversion.
/// Putting this logic in only one place makes it easier to edit the day where
/// we find a system on which that conversion breaks.
pub fn osstring_convert(origin: &OsStr) -> String {
    origin
        .to_str()
        .expect("OsStr -> String conversion failed")
        .to_string()
}
