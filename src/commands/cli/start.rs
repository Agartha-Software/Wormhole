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

pub fn start(ip: &str, mut start_args: StatusPodArgs) -> CliResult<String> {
    if start_args.name == "." {
        let p = env::current_dir()?;
        let path = WhPath::from(&p.display().to_string());
        start_args.path = if start_args.path.inner != "." {
            path.join(&start_args.path)
        } else {
            path
        }
    }

    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Start(start_args)))
}
