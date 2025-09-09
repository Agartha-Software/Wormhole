use crate::{
    commands::cli_commands::PodConf,
    config::{types::Config, GlobalConfig, LocalConfig},
    error::{CliError, CliResult, CliSuccess},
    pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
};
use parking_lot::RwLock;
use std::sync::Arc;

pub fn apply(
    local_config: Arc<RwLock<LocalConfig>>,
    global_config: Arc<RwLock<GlobalConfig>>,
    args: PodConf,
) -> CliResult<CliSuccess> {
    let mut log = String::default();
    for file in args.files {
        match file.as_str() {
            LOCAL_CONFIG_FNAME => {
                let mut conf = LocalConfig::read(
                    &args
                        .path
                        .as_ref()
                        .unwrap_or(&".".into())
                        .join(LOCAL_CONFIG_FNAME)
                        .inner,
                )?;
                {
                    let hostname = &local_config.read().general.hostname;
                    if conf.general.hostname != *hostname {
                        conf.general.hostname = hostname.clone();
                        log.push_str("Warning: hostname change rejected\n");
                    }
                    let url = &local_config.read().general.url;
                    if conf.general.url != *url {
                        conf.general.url = url.clone();
                        log.push_str("Warning: url change rejected\n");
                    }
                }
                *LocalConfig::write_lock(&local_config, "apply::local_config")? = conf;
            }
            GLOBAL_CONFIG_FNAME => {
                let conf = GlobalConfig::read(
                    &args
                        .path
                        .as_ref()
                        .unwrap_or(&".".into())
                        .join(GLOBAL_CONFIG_FNAME)
                        .inner,
                )?;
                *GlobalConfig::write_lock(&global_config, "apply::global_conf")? = conf;
            }
            _ => return Err(CliError::InvalidArgument { arg: file }),
        }
    }
    Ok(CliSuccess::Message(
        log.is_empty()
            .then_some("New configuration succesfully applied".into())
            .unwrap_or(format!("New configuration applied\n{log}")),
    ))
}
