// In rust we code
// In code we trust
// AgarthaSoftware - 2024

pub mod commands;
pub mod config;
pub mod data;


#[cfg(target_os = "linux")]
pub mod fuse;

#[cfg(target_os = "windows")]
pub mod winfsp;

pub mod network;
pub mod providers;

#[cfg(target_os = "windows")]
pub const INSTANCE_PATH: &str = "%APPDATA%/local/wormhole";

#[cfg(target_os = "linux")]
pub const INSTANCE_PATH: &'static str = "/usr/local/share/wormhole/";
