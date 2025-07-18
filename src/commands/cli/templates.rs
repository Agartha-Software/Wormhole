// In rust we code
// In code we trust
// AgarthaSoftware - 2024
use std::fs;

use crate::commands::{default_global_config, default_local_config};
use crate::config::types::Config;
use crate::error::CliResult;
use crate::pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME};
use crate::pods::whpath::WhPath;

#[must_use]
pub fn templates(path: &WhPath, name: &str) -> CliResult<()> {
    let global_config = default_global_config();
    let local_config = default_local_config(name);
    path.clone().set_absolute();
    fs::read_dir(path.inner.clone()).map(|_| ())?;
    local_config.write(path.join(LOCAL_CONFIG_FNAME).inner)?;
    global_config.write(path.join(GLOBAL_CONFIG_FNAME).inner)?;
    Ok(())
}
