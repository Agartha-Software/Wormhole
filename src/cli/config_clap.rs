use crate::cli::{IdentifyNewPodGroup, IdentifyPodGroup};
use clap::{ArgAction, ValueEnum};
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
    #[arg(long, short, default_value = "both", long = "type", short = 't')]
    pub config_type: ConfigType,
}

#[derive(Debug, Args, Clone)]
pub struct SaveConfigArgs {
    #[clap(flatten)]
    pub group: IdentifyNewPodGroup,
    /// Which configuration file
    #[arg(long, short, default_value = "both", long = "type", short = 't')]
    pub config_type: ConfigType,
    /// Overwrite existing files
    #[arg(long, short, action=ArgAction::SetTrue)]
    pub force: bool,
}

#[derive(Debug, Subcommand)]
#[command(version, name = "config")]
pub enum ConfigCommand {
    /// Write a pod configuration (global and/or local) to file, using defaults if the pod doesn’t exist
    Save(SaveConfigArgs),
    /// Show the configuration of a given pod
    Show(IdentifyPodAndConfigArgs),
    /// Validate that the configuration files of a pod are valid
    Check(IdentifyPodAndConfigArgs),
}
