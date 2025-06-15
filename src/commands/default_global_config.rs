use crate::config::{
    types::{GeneralGlobalConfig, RedundancyConfig},
    GlobalConfig,
};

pub fn default_global_config() -> GlobalConfig {
    return GlobalConfig {
        general: GeneralGlobalConfig {
            peers: Vec::new(),
            ignore_paths: Vec::new(),
            pods_names: Vec::new(),
        },
        redundancy: RedundancyConfig { number: 2 },
    };
}
