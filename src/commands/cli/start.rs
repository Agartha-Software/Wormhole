use std::env;

use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::path_or_wd,
        cli_commands::{
            Cli::{self},
            StatusPodArgs,
        },
    },
    error::CliResult,
    pods::whpath::WhPath,
};

use super::cli_messager;

pub fn start(ip: &str, mut start_args: StatusPodArgs) -> CliResult<String> {
    if start_args.name.is_none() {
        start_args.path = Some(path_or_wd(start_args.path)?)
    }

    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Start(start_args)))
}
