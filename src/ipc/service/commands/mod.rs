mod config;
mod freeze;
mod gethosts;
mod inspect;
mod new;
mod remove;
mod status;
mod tree;
mod unfreeze;

pub use config::show::show;
pub use config::validate::validate;
pub use config::write::write;
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

pub(self) fn find_pod<'a>(
    id: &'a PodId,
    pods: &'a HashMap<String, Pod>,
) -> Option<(&'a String, &'a Pod)> {
    match id {
        PodId::Name(name) => pods.get_key_value(name),
        PodId::Path(path) => pods
            .iter()
            .find(|(_, pod)| pod.get_mountpoint().as_str() == path.as_str()),
    }
}
