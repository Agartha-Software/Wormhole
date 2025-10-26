mod freeze;
mod gethosts;
mod inspect;
mod new;
mod remove;
mod status;
mod tree;
mod unfreeze;

pub use freeze::freeze;
pub use gethosts::gethosts;
pub use inspect::inspect;
pub use new::new;
pub use remove::remove;
pub use status::status;
pub use tree::tree;
pub use unfreeze::unfreeze;

use crate::ipc::commands::PodId;
use crate::pods::pod::Pod;
use crate::pods::whpath::JoinPath;
use std::collections::HashMap;

pub(self) fn find_pod(id: PodId, pods: &HashMap<String, Pod>) -> Option<(&String, &Pod)> {
    match id {
        PodId::Name(name) => pods.get_key_value(&name),
        PodId::Path(path) => pods
            .iter()
            .find(|(_, pod)| pod.get_mountpoint().as_str() == path.as_str()),
    }
}
