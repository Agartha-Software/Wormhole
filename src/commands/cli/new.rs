// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use std::{env, fs};

use tokio::runtime::Runtime;

use crate::{
    commands::{
        cli::message::cli_messager,
        cli_commands::{Cli, PodArgs},
        default_local_config,
    },
    config::{types::Config, LocalConfig},
    error::{CliError, CliResult},
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        whpath::WhPath,
    },
};

fn mod_file_conf_content(path: WhPath, name: String, ip: &str) -> Result<(), CliError> {
    let local_path = path.clone().join(LOCAL_CONFIG_FNAME).inner;
    let local_config = LocalConfig::read(&local_path).ok();
    let mut local_config = if let Some(local_config) = local_config {
        local_config
    } else {
        return Ok(());
    };
    if local_config.general.name != name {
        //REVIEW - Change the name without notifying the user or return an error? I think it would be better to return an error
        local_config.general.name = name.clone();
    }
    if ip != "127.0.0.1:8080" {
        local_config.general.address = ip.to_owned();
    }
    if let Err(_) = local_config.write(&local_path) {
        return Err(CliError::InvalidConfig { file: local_path });
    }
    Ok(())
}

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
    if args.path.inner == "." {
        args.path = WhPath::from(&env::current_dir()?.display().to_string());
    }

    mod_file_conf_content(args.path.clone(), args.name.clone(), &args.ip)?;
    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(
        ip,
        Cli::New(PodArgs {
            name: args.name,
            path: args.path.clone(),
            url: args.url,
            ip: args.ip,
            additional_hosts: args.additional_hosts,
        }),
    ))?;
    Ok("ok".to_string())
}
