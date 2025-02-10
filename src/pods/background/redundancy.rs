use std::{collections::HashMap, sync::Arc};

use tokio::sync::mpsc::UnboundedReceiver;

use crate::{network::message::Address, pods::{arbo::InodeId, fs_interface::FsInterface, network_interface::{Callbacks, NetworkInterface}}};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RedundancyMission {
    ino: InodeId,
    origin: Address,
    priority: u64,
}

struct RedundancyMissionResult {
    origin: Address,
    priority: u64,
}

fn resolve_case(answers: Vec<RedundancyMissionResult>, cb: Callbacks) {
    Callbacks.wait_for(cb);
}

pub async fn redundancy_watchdog(
    mut reception: UnboundedReceiver<RedundancyMission>,
    fsi: Arc<FsInterface>,
    nwi: Arc<NetworkInterface>,
) {
    let mut open_cases: HashMap<InodeId, Vec<RedundancyMissionResult>> = HashMap::new();

    loop {
        answers.push(match rx.recv().await {
            Some(message) => message.,
            None => continue,
        });
    }
}