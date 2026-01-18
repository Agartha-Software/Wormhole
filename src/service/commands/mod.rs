mod config;
mod freeze;
mod gethosts;
mod inspect;
mod new;
mod remove;
mod restart;
mod status;
mod tree;
mod unfreeze;

pub use config::apply::apply;
pub use config::check::check;
pub use config::generate::generate;
pub use config::show::show;
pub use gethosts::gethosts;
pub use inspect::inspect;
pub use new::new;
pub use status::status;
pub use tree::tree;

use crate::ipc::commands::PodId;
use crate::pods::{pod::Pod, prototype::PodPrototype};
use std::collections::HashMap;

pub(self) fn find_pod<'a>(
    id: &'a PodId,
    pods: &'a HashMap<String, Pod>,
) -> Option<(&'a String, &'a Pod)> {
    match id {
        PodId::Name(name) => pods.get_key_value(name),
        PodId::Path(path) => pods
            .iter()
            .find(|(_, pod)| pod.get_mountpoint().as_os_str() == path.as_os_str()),
    }
}

pub(self) fn find_frozen_pod<'a>(
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
