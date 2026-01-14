use crate::cli::{IdentifyNewPodGroup, IdentifyPodGroup};
use clap::{ArgAction, ValueEnum};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ValueEnum, TS)]
#[clap(rename_all = "lower")]
#[ts(export)]
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
pub struct GenerateConfigArgs {
    #[clap(flatten)]
    pub group: IdentifyNewPodGroup,
    /// Which configuration file
    #[arg(long, short, default_value = "both", long = "type", short = 't')]
    pub config_type: ConfigType,
    /// Overwrite files if they already exist
    #[arg(long, short, action=ArgAction::SetTrue)]
    pub force: bool,
}

#[derive(Debug, Subcommand)]
#[command(version, name = "config")]
pub enum ConfigCommand {
    /// Generate a pod configuration (global and/or local) to file, using defaults if the pod doesn’t exist
    Generate(GenerateConfigArgs),
    /// Show the configuration of a given pod
    Show(IdentifyPodAndConfigArgs),
    /// Check the configuration files of a pod to be valid
    Check(IdentifyPodAndConfigArgs),
    /// Apply the configuration file
    Apply(IdentifyPodAndConfigArgs),
}
