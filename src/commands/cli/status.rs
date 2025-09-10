// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use tokio::runtime::Runtime;

use crate::{
    commands::cli_commands::{Cli, GetHostsArgs},
    error::CliResult,
};

use super::cli_messager;

pub fn status(ip: &str) -> CliResult<String> {
    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Status))
}
