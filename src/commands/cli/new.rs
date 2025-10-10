// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use std::path::PathBuf;

use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::message::cli_messager,
        cli_commands::{Cli, PodArgs},
    },
    error::{CliError, CliResult},
};

//FIXME - Error id name of the pod not check (can be already exist)
pub fn new(ip: &str, mut args: PodArgs) -> CliResult<String> {
    match std::env::current_dir()
        .ok()
        .and_then(|f| -> Option<PathBuf> {
            f.join(args.mountpoint.clone().unwrap_or((&args.name).into()))
                .as_os_str()
                .try_into()
                .ok()
        }) {
        None => Err(CliError::InvalidArgument {
            arg: format!("path is invalid or missing"),
        }),
        Some(path) => {
            // mod_file_conf_content(path.clone(), args.hostname.clone(), &args.port)?;
            args.mountpoint = Some(path);
            let rt = Runtime::new().unwrap();
            rt.block_on(cli_messager(ip, Cli::New(args)))?;
            Ok("ok".to_string())
        }
    }
}
