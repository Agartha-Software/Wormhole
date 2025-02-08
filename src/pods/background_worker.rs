use std::sync::Arc;

use tokio::sync::mpsc::UnboundedReceiver;

use super::{fs_interface::FsInterface, network_interface::NetworkInterface};

pub enum BackgroundMission {
    End,
}

pub async fn background_worker_airport(
    mut reception: UnboundedReceiver<BackgroundMission>,
    fsi: Arc<FsInterface>,
    nwi: Arc<NetworkInterface>,
) {
    loop {
        let mission = match reception.recv().await {
            Some(message) => message,
            None => continue,
        };

        match mission {
            BackgroundMission::End => break,
        }
    }
}
