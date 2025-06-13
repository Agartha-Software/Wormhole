use std::{fs::File, path::Path, sync::Arc};

use parking_lot::RwLock;

use crate::{commands::cli_commands::PodConf,
    config::{types::Config, GlobalConfig, LocalConfig},
    error::{CliError, CliResult, CliSuccess},
    pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME}
};

pub fn restore(local_config: Arc<RwLock<LocalConfig>>, global_config: Arc<RwLock<GlobalConfig>>, args: PodConf) -> CliResult<CliSuccess> {
    let local_conf = LocalConfig::read_lock(&local_config, "service::restore::local")?;
    let global_conf = GlobalConfig::read_lock(&global_config, "service::restore::globale")?;
    for file in args.files {
        match file.as_str() {
            LOCAL_CONFIG_FNAME => {
                let path = args.path.join(LOCAL_CONFIG_FNAME).inner;
                if !Path::new(&path).exists() {
                    File::create(path.clone())?;
                }
                if let Err(e) = local_conf.write(path) {
                    return Err(CliError::BoxError { arg: e });
                }
            },
            GLOBAL_CONFIG_FNAME => {
                let path = args.path.join(GLOBAL_CONFIG_FNAME).inner;
                if !Path::new(&path).exists() {
                    File::create(path.clone())?;
                }
                if let Err(e) = global_conf.write(path) {
                    return Err(CliError::BoxError { arg: e });
                }
            },
            _ => {return Err(CliError::InvalidArgument { arg: file });}
        }
    }
    Ok(CliSuccess::Message("Configuration files are restore".to_owned()))
}