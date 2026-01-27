use std::{net::IpAddr, path::PathBuf};
use ts_rs::TS;

use crate::{
    cli::config_clap::ConfigCommand,
    pods::itree::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
    service::socket::SOCKET_DEFAULT_NAME,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None, name="wormhole")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
    /// Specify a specific service socket in case of multiple services running
    #[arg(short, long, default_value = SOCKET_DEFAULT_NAME)]
    pub socket: String,
}

#[derive(Debug, Subcommand)]
#[command(version, about, long_about = None, name="wormhole")]
pub enum CliCommand {
    /// Create a new pod and if possible join a network, otherwise create a new one
    New(NewArgs),
    /// Pause a given pod
    Freeze(IdentifyPodArgs),
    /// Unpause a given pod
    Unfreeze(IdentifyPodArgs),
    /// Restart a given pod
    Restart(IdentifyPodArgs),
    /// Remove a pod from its network and stop it
    Remove(RemoveArgs),
    /// Interact with the configuration of a pod (Write, Show, Validate)
    #[command(subcommand)]
    Config(ConfigCommand),
    /// Inspect the basic informations of a given pod
    Inspect(IdentifyPodArgs),
    /// Get the hosts of a given file
    GetHosts(GetHostsArgs),
    /// Display the file tree at a given pod or path and show the hosts for each files
    Tree(IdentifyPodArgs),
    /// Checks if the service is working
    Status,
    // /// Start the service
    // Start,
    // /// Stops the service
    // Stop,
}

fn canonicalize(path: PathBuf) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path)
}

fn parse_canonicalize(path_str: &str) -> std::io::Result<PathBuf> {
    canonicalize(PathBuf::from(path_str))
}

// like canonicalize but doesn't check if the final element exist
pub fn parse_canonicalize_non_existant(path_str: &str) -> std::io::Result<PathBuf> {
    let path = PathBuf::from(path_str);

    if path.exists() {
        return canonicalize(path);
    }

    let name = path.file_name().ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Path doesn't exist",
    ))?;

    let mut parent = match path.parent() {
        Some(parent) => canonicalize(parent.to_path_buf())?,
        None => std::env::current_dir()?,
    };
    parent.push(name);
    Ok(parent)
}

#[derive(Debug, Args, Clone)]
#[group(required = true, multiple = false)]
pub struct IdentifyPodGroup {
    /// Name of the pod
    pub name: Option<String>,
    /// Path of the pod
    #[arg(long, short, value_parser=parse_canonicalize)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
#[group(required = true, multiple = false)]
pub struct IdentifyNewPodGroup {
    /// Name of the pod
    pub name: Option<String>,
    /// Path of the pod
    #[arg(long, short, value_parser=parse_canonicalize_non_existant)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct IdentifyPodArgs {
    #[clap(flatten)]
    pub group: IdentifyPodGroup,
}

#[derive(Debug, Args, Clone)]
#[command(about, long_about = None)]
pub struct PodConfArgs {
    #[clap(flatten)]
    pub group: IdentifyPodGroup,

    /// Names of all configuration files that you want to restore
    #[arg(long, short, default_values_t = [String::from(LOCAL_CONFIG_FNAME), String::from(GLOBAL_CONFIG_FNAME)])]
    pub files: Vec<String>,
}

#[derive(Debug, Args, Clone)]
#[command(about, long_about = None)]
pub struct GetHostsArgs {
    /// Path of the file
    #[arg(required = true, value_parser=parse_canonicalize)]
    pub path: PathBuf,
}

#[derive(Debug, Args, Clone)]
#[command(about, long_about = None)]
pub struct NewArgs {
    /// Name of the pod to create
    // TODO: make optional again when the url can provide the name expected
    pub name: String,
    /// Mount point to create the pod in. By default creates a mount point in the working directory with the name of the pod
    #[arg(long = "mount", short, value_parser=parse_canonicalize_non_existant)]
    pub mountpoint: Option<PathBuf>,
    /// Network to join
    #[arg(long, short)]
    pub url: Option<String>,
    /// nickname name to report to the network
    /// defaults to your machine's hostname + ':' + name of the pod
    #[arg(long, short='k')]
    pub nickname: Option<String>,
    /// Ip address this Pod listen [default: 127.0.0.1]
    #[arg(long, short)]
    pub ip_address: Option<IpAddr>,
    /// Local port for the pod to use. By default automatically find a port on the range [default: 40000-40100]
    #[arg(long, short)]
    pub port: Option<u16>,
    /// Additional hosts to try to join from as a backup
    #[arg(raw = true)]
    pub additional_hosts: Vec<String>,
    /// Allow other users to access the mounted pod
    #[arg(short, long, default_value_t = false)]
    pub allow_other_users: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ValueEnum, TS)]
#[clap(rename_all = "lower")]
#[ts(export)]
pub enum Mode {
    /// Simply remove the pod from the network without losing any data from the network
    /// and leaving behind any data that was stored on the pod
    Simple,
    /// Remove the pod from the network without losing any data on the network,
    /// and clone all data from the network into the folder where the pod was
    /// making this folder into a real folder
    Clone,
    /// Remove the pod from the network and delete any data that was stored in the pod
    Clean,
    /// Remove this pod from the network without distributing its data to other nodes
    Take,
}

// Structure RemoveArgs modifiée
#[derive(Debug, Args, Clone)]
#[command(about, long_about = None)]
pub struct RemoveArgs {
    #[clap(flatten)]
    pub group: IdentifyPodGroup,
    ///// Mode for pod removal
    // #[arg(long, default_value = "simple")]
    // pub mode: Mode,
}
