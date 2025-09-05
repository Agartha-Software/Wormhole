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
    for file in args.files {
        match file.as_str() {
            LOCAL_CONFIG_FNAME => {
                let conf = LocalConfig::read(
                    &args
                        .path
                        .as_ref()
                        .unwrap_or(&".".into())
                        .join(LOCAL_CONFIG_FNAME)
                        .inner,
                )?;
                LocalConfig::write_lock(&local_config, "apply::local_config")?.constructor(conf)?;
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
                GlobalConfig::write_lock(&global_config, "apply::global_conf")?
                    .constructor(conf)?;
            }
            _ => return Err(CliError::InvalidArgument { arg: file }),
        }
    }
    Ok(CliSuccess::Message(
        "The new configuration is apply".to_owned(),
    ))
}
