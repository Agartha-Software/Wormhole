// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::path_or_wd,
        cli_commands::{Cli, TreeArgs},
    },
    error::CliResult,
};

use super::cli_messager;

pub fn tree(ip: &str, mut args: TreeArgs) -> CliResult<String> {
    if args.name.is_none() {
        args.path = Some(path_or_wd(args.path)?)
    }

    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Tree(args)))
}
