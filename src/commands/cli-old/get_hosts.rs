// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::path_or_wd,
        cli_commands::{Cli, GetHostsArgs},
    },
    error::CliResult,
};

use super::cli_messager;

pub fn get_hosts(ip: &str, mut args: GetHostsArgs) -> CliResult<String> {
    if args.name.is_none() {
        args.path = Some(path_or_wd(args.path)?)
    }
    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::GetHosts(args)))
}
