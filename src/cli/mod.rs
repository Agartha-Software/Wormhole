//// module for everything cli related except for ipc with the service

mod clap;
mod config_clap;
mod display;

pub use clap::*;
pub use config_clap::*;
pub use display::print_err;
