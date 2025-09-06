use crate::{
    commands::cli_commands::PodArgs,
    config::{types::Config, GlobalConfig, LocalConfig},
    error::{CliError, CliResult},
    network::server::Server,
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::Pod,
        whpath::WhPath,
    },
};
use gethostname::gethostname;
use std::{path::PathBuf, sync::Arc};

pub async fn new(args: PodArgs) -> CliResult<Pod> {
    let (global_config, local_config, server, mount_point) = pod_value(&args).await?;
    Pod::new(
        // local_config.general.name.clone(),
        global_config,
        local_config.clone(),
        mount_point,
        server.clone(),
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

    additional_hosts.insert(0, url);
    global_config.general.entrypoints.extend(additional_hosts);
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

    let address = "0.0.0.0:".to_owned() + &args.port;
    let local_cfg_path = path.join(LOCAL_CONFIG_FNAME);
    let global_cfg_path = path.join(GLOBAL_CONFIG_FNAME);

    let mut local_config: LocalConfig = LocalConfig::read(&local_cfg_path).unwrap_or_default();
    local_config.general.hostname = args
        .hostname
        .clone()
        .or_else(|| gethostname().into_string().ok())
        .ok_or(CliError::Message {
            reason: "no valid hostname".to_owned(),
        })?;
    local_config.general.url = Some(
        args.listen_url
            .as_ref()
            .unwrap_or(&local_config.general.hostname)
            .clone(),
    );
    let server: Arc<Server> = Arc::new(Server::setup(&address).await?);

    let global_config: GlobalConfig = GlobalConfig::read(global_cfg_path).unwrap_or_default();
    let global_config = add_hosts(
        global_config,
        args.url.clone().unwrap_or("".to_string()),
        args.additional_hosts.clone(),
    );

    Ok((global_config, local_config, server, path.as_os_str().into()))
}
