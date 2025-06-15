// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use tokio::runtime::Runtime;

use crate::{
    commands::cli_commands::{Cli, TreeArgs},
    error::CliResult,
};

use super::cli_messager;

pub fn tree(ip: &str, args: TreeArgs) -> CliResult<()> {
    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Tree(args)))
}
