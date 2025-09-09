use futures::io::Read;

use crate::{
    commands::cli_commands::{Mode, RemoveArgs},
    config::{types::Config, LocalConfig},
    error::{CliError, CliResult, CliSuccess, WhError},
    network::message::{Address, MessageContent, ToNetworkMessage},
    pods::{
        arbo::{Arbo, FsEntry, ARBO_FILE_FNAME},
        filesystem::{fs_interface, read::ReadError},
        network::pull_file::PullError,
        pod::Pod,
    },
};
use std::path::Path;

/// Remove a pod from the network with different modes
pub async fn remove(args: RemoveArgs, pod: Pod) -> CliResult<CliSuccess> {
    let pod_name = pod.get_name().to_string();
    let mount_point = pod.get_mount_point().clone().inner;

    match args.mode {
        Mode::Simple => simple_remove(pod).await,
        Mode::Clone => clone_remove(pod).await,
        Mode::Clean => clean_remove(pod).await,
        Mode::Take => take_remove(pod).await,
        Mode::Freeze => freeze_remove(pod).await,
    }
    .map(|_| {
        CliSuccess::Message(format!(
            "Pod '{}' at '{}' removed successfully with mode '{:?}'",
            pod_name, mount_point, args.mode
        ))
    })
}

// SECTION ---------------------------------------------- Simple Remove -----------------------

/// Mode Simple: Remove the pod from the network without losing any data from the network
/// and leaving behind any data that was stored on the pod
async fn simple_remove(pod: Pod) -> Result<(), CliError> {
    pod.stop()
        .await
        .map_err(|e| CliError::PodStopError { source: e })?;

    Ok(())
}

// SECTION ---------------------------------------------- Clean Remove -----------------------

async fn clean_remove(_pod: Pod) -> Result<(), CliError> {
    Err(CliError::Unimplemented {
        arg: "Clean remove".into(),
    })
}

// SECTION ---------------------------------------------- Clone Remove -----------------------

/// Remove the pod from the network without losing any data on the network,
/// and clone all data from the network into the folder where the pod was
/// making this folder into a real folder
async fn clone_remove(pod: Pod) -> Result<(), CliError> {
    let pod_address = {
        let local_config = LocalConfig::read_lock(&pod.local_config, "remove_clone")?;
        local_config.general.address.clone()
    };
    let arbo = Arbo::n_read_lock(&pod.network_interface.arbo, "remove_clone")?;
    clone_missing_files_from_network(&pod, &arbo, pod_address.to_string()).await?;
    drop(arbo);
    pod.stop()
        .await
        .map_err(|e| CliError::PodStopError { source: e })?;
    Ok(())
}

/// Clone missing files from the network to make the folder complete
async fn clone_missing_files_from_network(
    pod: &Pod,
    arbo: &Arbo,
    pod_address: String,
) -> Result<(), CliError> {
    let missing_files: Vec<_> = arbo
        .iter()
        .filter_map(|(_, inode)| match &inode.entry {
            FsEntry::File(hosts) if !hosts.contains(&pod_address) && !hosts.is_empty() => {
                Some(inode.id)
            }
            _ => None,
        })
        .collect();

    if missing_files.is_empty() {
        return Ok(());
    }
    log::info!(
        "Found {} missing files to clone from network",
        missing_files.len()
    );
    for file_ino in missing_files {
        let ok = match pod.network_interface.pull_file_sync(file_ino)? {
            None => true,
            Some(callback) => pod.network_interface.callbacks.n_wait_for(callback)?,
        };
        if !ok {
            return Err(CliError::ReadError {
                source: ReadError::CantPull,
            });
        }
    }

    Ok(())
}

// SECTION ---------------------------------------------- Take Remove -----------------------

async fn take_remove(_pod: Pod) -> Result<(), CliError> {
    Err(CliError::Unimplemented {
        arg: "Take remove".into(),
    })
}

// SECTION ---------------------------------------------- Freeze Remove -----------------------

async fn freeze_remove(_pod: Pod) -> Result<(), CliError> {
    Err(CliError::Unimplemented {
        arg: "Freeze remove".into(),
    })
}
