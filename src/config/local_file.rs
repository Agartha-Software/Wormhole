use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocalConfigFile {
    pub general: GeneralLocalConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeneralLocalConfig {
    pub name: Option<String>,
    pub hostname: Option<String>,
    pub public_url: Option<String>,
    pub restart: Option<bool>,
}
