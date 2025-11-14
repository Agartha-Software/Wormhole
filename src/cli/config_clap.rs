use crate::cli::{IdentifyNewPodArgs, IdentifyPodGroup};
use clap::ValueEnum;
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum ConfigType {
    Local,
    Global,
    Both,
}

impl ConfigType {
    pub fn is_local(&self) -> bool {
        matches!(self, ConfigType::Local | ConfigType::Both)
    }

    pub fn is_global(&self) -> bool {
        matches!(self, ConfigType::Global | ConfigType::Both)
    }
}

#[derive(Debug, Args, Clone)]
pub struct IdentifyPodAndConfigArgs {
    #[clap(flatten)]
    pub group: IdentifyPodGroup,
    /// Which configuration file
    #[arg(default_value = "both", name = "TYPE")]
    pub config_type: ConfigType,
}

#[derive(Debug, Subcommand)]
#[command(version, name = "config")]
pub enum ConfigCommand {
    /// Write a pod configuration (global and local) to file, using defaults if the pod doesn’t exist
    Write(IdentifyNewPodArgs),
    /// Show the configuration of a given pod
    Show(IdentifyPodAndConfigArgs),
    /// Validate that the configuration files of a pod have a valid format
    Validate(IdentifyPodAndConfigArgs),
}
