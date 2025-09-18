use serde::{Deserialize, Serialize};

use crate::pods::whpath::WhPath;

#[derive(Debug, clap::Args, Serialize, Deserialize, Clone)]
pub struct PodCreationArgs {
    /// Name of the pod
    pub name: String,
    /// mount point to create the pod in. By default creates a pod from the folder in the working directory with the name of the pod
    pub mountpoint: Option<WhPath>,
    /// Local port for the pod to use
    pub port: String,
    /// Network to join
    pub url: Option<String>,
    /// Name for this pod to use as a machine name with the network. Defaults to your Machine's name
    pub hostname: Option<String>,
    /// url this Pod reports to other to reach it
    pub listen_url: Option<String>,
    /// Additional hosts to try to join from as a backup
    pub additional_hosts: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum RemoveMode {
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

#[derive(Debug, Serialize, Deserialize)]
pub enum CliMode {
    ToDelete,
    Oneshot,
    Maintain,
}

type PodInfo = (Option<String>, Option<WhPath>);
#[derive(Debug, Serialize, Deserialize)]
pub enum CliRequest {
    /// First message of the cli client to connect
    Register(CliMode),
    /// Last message of the cli client to disconnect
    Close,
    /// Start the pod (name, path)
    StartPod(PodInfo),
    /// Stop the pod (name, path)
    StopPod(PodInfo),
    /// Create a new pod and join a network if he have peers in arguments or create a new network
    New(PodCreationArgs),
    /// Inspect a pod with its configuration, connections, etc
    Inspect(PodInfo),
    /// Get hosts for a specific file
    GetHosts(PodInfo),
    /// Tree the folder structure from the given path and show hosts for each file
    Tree(PodInfo),
    /// Checks that the service is working (should print it's ip)
    ServiceStatus,
    /// Remove a pod from its network
    Remove(PodInfo, RemoveMode),
    /// Apply a new configuration to a pod
    Apply(PodInfo, Vec<String>),
    /// Restore many or a specific file configuration
    Restore(PodInfo, Vec<String>),
    /// Stops the service
    Interrupt,
}

#[derive(Debug, Serialize, Deserialize)] // requires `derive` feature
pub enum CliAnswer {
    //StartPod(Result<todo!(), todo!()>),
    /// Error (only about the comm between client and server, not about the command)
    Error(String),
}
