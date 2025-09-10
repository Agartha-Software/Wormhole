// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use std::fs;

use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::message::cli_messager,
        cli_commands::{Cli, PodArgs},
    },
    config::{types::Config, LocalConfig},
    error::{CliError, CliResult},
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        whpath::WhPath,
    },
};

// fn mod_file_conf_content(path: WhPath, hostname: String) -> Result<(), CliError> {
//     let local_path = path.clone().join(LOCAL_CONFIG_FNAME).inner;
//     let local_config = LocalConfig::read(&local_path).ok();
//     let mut local_config = if let Some(local_config) = local_config {
//         local_config
//     } else {
//         return Ok(());
//     };
//     local_config.general.hostname = hostname.clone();
//     if let Err(_) = local_config.write(&local_path) {
//         return Err(CliError::InvalidConfig { file: local_path });
//     }
//     Ok(())
// }

fn is_new_wh_file_config(path: &WhPath) -> CliResult<()> {
    let files_name = vec![LOCAL_CONFIG_FNAME, GLOBAL_CONFIG_FNAME];
    for file_name in files_name {
        if fs::metadata(path.clone().push(file_name).inner.clone()).is_err() {
            return Err(CliError::FileConfigName {
                name: file_name.to_owned(),
            });
        }
    }
    Ok(())
}

//FIXME - Error id name of the pod not check (can be already exist)
pub fn new(ip: &str, mut args: PodArgs) -> CliResult<String> {
    match std::env::current_dir()
        .ok()
        .and_then(|f| -> Option<WhPath> {
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
