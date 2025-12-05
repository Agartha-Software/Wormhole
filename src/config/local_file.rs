use serde::{Deserialize, Serialize};

use crate::error::CliError;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocalConfigFile {
    pub general: GeneralLocalConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeneralLocalConfig {
    pub name: Option<String>,
    pub port: Option<u16>,
    pub hostname: Option<String>,
    pub url: Option<String>,
}

impl LocalConfigFile {
    pub fn constructor(&mut self, local: Self) -> Result<(), CliError> {
        // self.general.name = local.general.name;
        if local.general.hostname != self.general.hostname {
            log::warn!("Local Config: Impossible to modify an ip address");
            return Err(CliError::Unimplemented {
                arg: "Local Config: Impossible to modify an ip address".to_owned(),
            });
        }
        Ok(())
    }
}
