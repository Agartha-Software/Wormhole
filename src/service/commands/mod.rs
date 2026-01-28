mod config;
mod freeze;
mod gethosts;
mod inspect;
mod list_pods;
mod new;
mod redundancy_status;
mod remove;
mod restart;
mod stats_per_filetype;
mod status;
mod tree;
mod unfreeze;

use crate::ipc::commands::PodId;
use crate::pods::pod::Pod;
use std::collections::HashMap;

use crate::pods::prototype::PodPrototype;

fn find_pod<'a>(id: &'a PodId, pods: &'a HashMap<String, Pod>) -> Option<(&'a String, &'a Pod)> {
    match id {
        PodId::Name(name) => pods.get_key_value(name),
        PodId::Path(path) => pods
            .iter()
            .find(|(_, pod)| pod.get_mountpoint().as_os_str() == path.as_os_str()),
    }
}

pub fn find_frozen_pod<'a>(
    id: &'a PodId,
    pods: &'a HashMap<String, PodPrototype>,
) -> Option<(&'a String, &'a PodPrototype)> {
    match id {
        PodId::Name(name) => pods.get_key_value(name),
        PodId::Path(path) => pods
            .iter()
            .find(|(_, pod)| pod.mountpoint.as_os_str() == path.as_os_str()),
    }
}
