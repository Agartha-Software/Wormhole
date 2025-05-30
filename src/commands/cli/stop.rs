use std::env;

use tokio::runtime::Runtime;

use crate::{
    commands::cli_commands::{
        Cli::{self},
        StatusPodArgs,
    },
    error::CliResult,
    pods::whpath::WhPath,
};

use super::cli_messager;

pub fn stop(ip: &str, mut stop_args: StatusPodArgs) -> CliResult<()> {
    if stop_args.name == "." {
        let p = env::current_dir()?;
        let path = WhPath::from(&p.display().to_string());
        stop_args.path = if stop_args.path.inner != "." {
            path.join(&stop_args.path)
        } else {
            path
        }
    }

    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Stop(stop_args)))
}
