use clap::Subcommand;

use crate::cli::IdentifyNewPodArgs;

#[derive(Debug, Subcommand)]
#[command(version, name = "config")]
pub enum ConfigCommand {
    /// Write a pod configuration (global and local) to file, using defaults if the pod doesn’t exist
    Write(IdentifyNewPodArgs),
    /// Show the configuration of a given pod
    Show,
    /// Validate that the configuration files of a pod have a valid format
    Validate,
}
