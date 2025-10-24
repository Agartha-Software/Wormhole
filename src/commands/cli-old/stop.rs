use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::path_or_wd,
        cli_commands::{
            Cli::{self},
            IdentifyPodArgs,
        },
    },
    error::CliResult,
};

use super::cli_messager;

pub fn stop(ip: &str, mut stop_args: IdentifyPodArgs) -> CliResult<String> {
    if stop_args.name.is_none() {
        stop_args.path = Some(path_or_wd(stop_args.path)?)
    }

    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Stop(stop_args)))
}
