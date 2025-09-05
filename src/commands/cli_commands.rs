use crate::pods::{
    arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
    whpath::WhPath,
};
use clap::{Args, Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser, Serialize, Deserialize)] // requires `derive` feature
#[command(name = "wormhole")]
#[command(bin_name = "wormhole")]
pub enum Cli {
    /// Start the service
    Start(StatusPodArgs),
    /// Stop the service
    Stop(StatusPodArgs),
    /// Create a new network (template)
    Template(TemplateArg),
    /// Create a new pod and join a network if he have peers in arguments or create a new network
    New(PodArgs),
    /// Inspect a pod with its configuration, connections, etc
    Inspect,
    /// Get hosts for a specific file
    GetHosts(GetHostsArgs),
    /// Tree the folder structure from the given path and show hosts for each file
    Tree(TreeArgs),
    /// Checks that the service is working (should print it's ip)
    Status,
    /// Remove a pod from its network
    Remove(RemoveArgs),
    /// Apply a new configuration to a pod
    Apply(PodConf),
    /// Restore many or a specifique file configuration  
    Restore(PodConf),
    /// Stops the service
    Interrupt,
}

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
#[command(version, about, long_about = None)]
pub struct PodConf {
    /// Pod name
    #[arg(long, short, default_value = ".")]
    pub name: String,
    /// Path of the pod
    #[arg(long, short, default_value = ".")]
    pub path: WhPath,
    /// Names of all configuration files that you want to restore
    #[arg(long, short, default_values_t = [String::from(LOCAL_CONFIG_FNAME), String::from(GLOBAL_CONFIG_FNAME)])]
    pub files: Vec<String>,
}

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
#[command(version, about, long_about = None)]
pub struct GetHostsArgs {
    /// Name of the pod
    #[arg(long, short)]
    pub name: String,
    /// File path from the root of the wh folder
    #[arg(long, short)]
    pub path: WhPath,
}

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
#[command(version, about, long_about = None)]
pub struct TreeArgs {
    /// Name of the pod
    #[arg(long, short)]
    pub name: String,
    /// Root of the tree
    #[arg(long, short, default_value = "/")]
    pub path: WhPath,
}

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
#[command(version, about, long_about = None)]
pub struct PodArgs {
    /// Name of the pod
    #[arg(long, short)]
    pub name: String,
    /// Change to DIRECTORY before doing anything
    #[arg(long, short, default_value = ".")]
    pub path: WhPath,
    /// Modify the default ip address of the Pod
    #[arg(long, short, default_value = "127.17.0.1:8081")]
    pub ip: String,
    /// Network url as <address of node to join from> + ':' + <network name>'
    #[arg(long, short)]
    pub url: Option<String>,
    /// Additional hosts to try to join from as a backup
    #[arg(long, short)]
    pub additional_hosts: Option<Vec<String>>,
}

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
#[command(version, about, long_about = None)]
pub struct StatusPodArgs {
    /// Name of the pod for updating status pod. If the name equal 'None' the name will be read from the current directory
    #[arg(long, short, default_value = ".")]
    pub name: String,
    /// Path is used uniquely if the pod name is 'None'
    #[arg(long, short, default_value = ".")]
    pub path: WhPath,
}

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
#[command(version, about, long_about = None)]
pub struct TemplateArg {
    /// Name of the network to create
    #[arg(long, short, default_value = "default")]
    pub name: String,
    /// Change to DIRECTORY before doing anything
    #[arg(long, short, default_value = ".")]
    pub path: WhPath,
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
#[command(version, about, long_about = None)]
pub struct RemoveArgs {
    /// Name of the deleted pod
    #[arg(long, short, required_unless_present = "path", conflicts_with = "path")]
    pub name: String,
    /// Change to DIRECTORY before doing anything
    #[arg(long, short, required_unless_present = "name", conflicts_with = "name")]
    pub path: WhPath,
    /// Mode for pod removal
    #[arg(long, default_value = "simple")]
    pub mode: Mode,
}
