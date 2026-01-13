mod config;
mod freeze;
mod gethosts;
mod inspect;
mod new;
mod remove;
mod status;
mod list_pods;
mod tree;
mod unfreeze;
mod redundancy_status;

pub use config::check::check;
pub use config::generate::generate;
pub use config::show::show;
pub use freeze::freeze;
pub use gethosts::gethosts;
pub use inspect::inspect;
pub use new::new;
pub use remove::remove;
pub use status::status;
pub use list_pods::list_pods;
pub use tree::tree;
pub use unfreeze::unfreeze;
pub use redundancy_status::redundancy_status;

use crate::ipc::commands::PodId;
use crate::pods::pod::Pod;
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
