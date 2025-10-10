// In rust we code
// In code we trust
// AgarthaSoftware - 2024
use std::fs;
use std::path::PathBuf;

use crate::config::types::Config;
use crate::config::{GlobalConfig, LocalConfig};
use crate::error::CliResult;
use crate::pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME};

#[must_use]
pub fn templates(path: &PathBuf, _name: &str) -> CliResult<String> {
    // TODO - name unused
    let global_config = GlobalConfig::default();
    let local_config = LocalConfig::default();
    fs::read_dir(path).map(|_| ())?;
    local_config.write(&path.join(LOCAL_CONFIG_FNAME))?;
    global_config.write(&path.join(GLOBAL_CONFIG_FNAME))?;
    Ok("ok".to_string())
}
