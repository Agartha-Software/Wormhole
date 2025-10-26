use std::path::PathBuf;

use crate::{
    ipc::service::SOCKET_DEFAULT_NAME,
    pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None, name="wormhole")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
    /// Specify a specific service socket in case of multiple services running, Defaults to 'wormhole.sock'.
    #[arg(short, long, default_value = SOCKET_DEFAULT_NAME)]
    pub socket: String,
}

#[derive(Debug, Subcommand)]
#[command(version, about, long_about = None, name="wormhole")]
pub enum CliCommand {
    /// Create a new pod and join a network if possible or create a new network
    New(NewArgs),
    /// Pause a given pod
    Freeze(IdentifyPodArgs),
    /// Restart a given pod
    UnFreeze(IdentifyPodArgs),
    /// Create a new network (template)
    Template(TemplateArg),
    /// Inspect a pod with its configuration, connections, etc...
    Inspect(IdentifyPodArgs),
    /// Get hosts for a specific file
    GetHosts(GetHostsArgs),
    /// Tree the folder structure of the given path and show hosts for each file
    Tree(IdentifyPodArgs),
    /// Remove a pod from its network
    Remove(RemoveArgs),
    /// Apply a new configuration to a pod
    Apply(PodConfArgs),
    /// Restore many or a specific file configuration
    Restore(PodConfArgs),
    /// Checks that the service is working (print it's ip)
    Status,
    /// Start the service
    Start,
    /// Stops the service
    Stop,
}

fn canonicalize(path_str: &str) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(PathBuf::from(path_str))
}

#[derive(Debug, Args, Serialize, Deserialize, Clone)]
#[group(required = true, multiple = false)]
pub struct IdentifyPodGroup {
    /// Name of the pod. Takes precedence over path
    #[arg(long, short)]
    pub name: Option<String>,
    /// Path of the pod, defaults to working directory
    #[arg(long, short, value_parser=canonicalize)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args, Serialize, Deserialize, Clone)]
#[command(about, name = "start")]
pub struct IdentifyPodArgs {
    #[clap(flatten)]
    pub group: IdentifyPodGroup,
}

#[derive(Debug, Args, Serialize, Deserialize, Clone)]
#[command(about, long_about = None)]
pub struct PodConfArgs {
    #[clap(flatten)]
    pub group: IdentifyPodGroup,

    /// Names of all configuration files that you want to restore
    #[arg(long, short, default_values_t = [String::from(LOCAL_CONFIG_FNAME), String::from(GLOBAL_CONFIG_FNAME)])]
    pub files: Vec<String>,
}

#[derive(Debug, Args, Serialize, Deserialize, Clone)]
#[command(about, long_about = None)]
pub struct GetHostsArgs {
    /// Path of the file
    #[arg(required = true, value_parser=canonicalize)]
    pub path: PathBuf,
}

#[derive(Debug, Args, Serialize, Deserialize, Clone)]
#[command(about, long_about = None)]
pub struct NewArgs {
    /// Name of the pod to create
    // TODO: make optional again when the url can provide the name expected
    pub name: String,
    /// Local port for the pod to use. By default automatically find a port on the range [40000-40100]
    #[arg(long, short)]
    pub port: Option<u16>,
    /// Mount point to create the pod in. By default creates a mount point in the working directory with the name of the pod
    #[arg(long = "mount", short, value_parser=canonicalize)]
    pub mountpoint: Option<PathBuf>,
    /// Network to join
    #[arg(long, short)]
    pub url: Option<String>,
    /// Name for this pod to use as a machine name with the network. Defaults to your Machine's name
    #[arg(long, short = 'H')]
    pub hostname: Option<String>,
    /// url this Pod reports to other to reach it
    #[arg(long, short)]
    pub listen_url: Option<String>,
    /// Additional hosts to try to join from as a backup
    #[arg(raw = true)]
    pub additional_hosts: Vec<String>,
}

#[derive(Debug, Args, Serialize, Deserialize, Clone)]
#[command(about, long_about = None)]
pub struct TemplateArg {
    /// Name of the pod to create
    pub name: String,
    /// Mount point to create the pod in. By default creates a mount point in the working directory with the name of the pod
    #[arg(long = "mount", short, value_parser=canonicalize)]
    pub mountpoint: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ValueEnum)]
#[clap(rename_all = "lower")]
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
#[derive(Debug, Args, Serialize, Deserialize, Clone)]
#[command(about, long_about = None)]
pub struct RemoveArgs {
    /// Name of pod to delete. Takes precedence over path
    #[arg(long, short, required_unless_present = "path", conflicts_with = "path")]
    pub name: Option<String>,
    /// Path of the pod to remove
    #[arg(long, short, required_unless_present = "name", conflicts_with = "name")]
    pub path: Option<PathBuf>,
    /// Mode for pod removal
    #[arg(long, default_value = "simple")]
    pub mode: Mode,
}
