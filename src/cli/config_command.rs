use std::path::PathBuf;

use crate::cli::parse_canonicalize_non_existant;
use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
#[command(version, name = "config")]
pub enum ConfigCommand {
    /// Write a pod configuration (global and local) to file, using defaults if the pod doesn’t exist
    Write(WriteConfigArg),
    /// Show the configuration of a given pod
    Show,
    /// Validate that the configuration files of a pod have a valid format
    Validate,
}

#[derive(Debug, Args, Clone)]
#[group(required = true, multiple = false, value_parser=test)]
pub struct WriteConfigArgGroup {
    /// Name of a pod
    pub name: Option<String>,
    /// Path of a pod or where to generate the default configuration for a pod
    #[arg(long, short, value_parser=parse_canonicalize_non_existant)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct WriteConfigArg {
    #[clap(flatten)]
    pub group: WriteConfigArgGroup,
}
