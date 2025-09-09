use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::path_or_wd,
        cli_commands::{Cli, PodConf},
    },
    error::{CliError, CliResult},
    pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
};

use super::cli_messager;

pub fn apply(ip: &str, mut args: PodConf) -> CliResult<String> {
    let files_name = vec![LOCAL_CONFIG_FNAME, GLOBAL_CONFIG_FNAME];

    for file in args.files.clone() {
        if !files_name.contains(&file.as_str()) {
            return Err(CliError::FileConfigName { name: file });
        }
    }
    if args.name.is_none() {
        args.path = Some(path_or_wd(args.path)?)
    }
    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(
        ip,
        Cli::Apply(PodConf {
            name: args.name,
            path: args.path,
            files: args.files,
        }),
    ))?;
    Ok("ok".to_string())
}
