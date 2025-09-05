use crate::{
    commands::{cli_commands::PodArgs, default_global_config, default_local_config},
    config::{types::Config, GlobalConfig, LocalConfig},
    error::{CliError, CliResult},
    network::server::Server,
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::Pod,
        whpath::WhPath,
    },
};
use std::{path::PathBuf, sync::Arc};

pub async fn new(args: PodArgs) -> CliResult<Pod> {
    let (global_config, local_config, server, mount_point) = pod_value(&args).await?;
    Pod::new(
        local_config.general.name.clone(),
        global_config,
        local_config.clone(),
        mount_point,
        server.clone(),
        local_config.clone().general.address,
    )
    .await
    .map_err(|e| CliError::PodCreationFailed { reason: e })
    .inspect_err(|e| log::error!("Pod creation failed: {}", e))
}

fn add_hosts(
    mut global_config: GlobalConfig,
    url: String,
    mut additional_hosts: Vec<String>,
) -> GlobalConfig {
    if url.is_empty() && additional_hosts.is_empty() {
        return global_config;
    }

    additional_hosts.push(url);
    if global_config.general.peers.is_empty() {
        global_config.general.peers = additional_hosts;
    } else {
        for host in additional_hosts {
            if !global_config.general.peers.contains(&host) {
                global_config.general.peers.push(host);
            }
        }
    }
    global_config
}

async fn pod_value(args: &PodArgs) -> CliResult<(GlobalConfig, LocalConfig, Arc<Server>, WhPath)> {
    log::info!("args: {args:?}");
    let path = args
        .mountpoint
        .as_ref()
        .and_then(|path| {
            let (parent, folder) = path.split_folder_file();
            std::fs::canonicalize(&parent)
                .ok()
                .map(|p| PathBuf::from(p).join(folder))
        })
        .ok_or(CliError::InvalidArgument {
            arg: "path".to_owned(),
        })?;

    log::info!("canonical: {:?}", path);
    // let pod_name = args.name.clone().ok_or(||CliError::PodNotFound).or_else(|_| match std::fs::canonicalize(&path).map(|file| {
    //     file.file_name()
    //     .and_then(|f| f.to_str())
    //     .map(|f| f.to_owned())
    // }) {
    //     Ok(Some(name)) => Ok(name.to_owned()),
    //     e => {
    //         Err(CliError::InvalidArgument {
    //             arg: format!("name: {e:?}"),
    //         })
    //     }
    // })?;
    // log::info!("name: {pod_name}");
    let address = "0.0.0.0:".to_owned() + &args.port;
    let local_cfg_path = path.join(LOCAL_CONFIG_FNAME);
    let global_cfg_path = path.join(GLOBAL_CONFIG_FNAME);

    // return Err(CliError::BincodeError);

    let mut local_config: LocalConfig =
        LocalConfig::read(&local_cfg_path).unwrap_or(default_local_config(&args.name));
    if local_config.general.name != args.name {
        //REVIEW - Change the name without notifying the user or return an error? I think it would be better to return an error
        local_config.general.name = args.name.clone();
    }
    if local_config.general.address != address {
        local_config.general.address = address;
    }
    let server: Arc<Server> = Arc::new(Server::setup(&local_config.general.address).await?);

    let global_config: GlobalConfig =
        GlobalConfig::read(global_cfg_path).unwrap_or(default_global_config());
    let global_config = add_hosts(
        global_config,
        args.url.clone().unwrap_or("".to_string()),
        args.additional_hosts.clone(),
    );

    Ok((global_config, local_config, server, path.as_os_str().into()))
}
