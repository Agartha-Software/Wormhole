use crate::config::types::Config;
use crate::config::LocalConfig;
use crate::error::WhResult;
use crate::network::message::{MessageContent, ToNetworkMessage};
use crate::pods::arbo::{Arbo, FsEntry};
use crate::pods::network::callbacks::Callback;
use crate::pods::network::network_interface::NetworkInterface;
use crate::{error::WhError, pods::arbo::InodeId};
use custom_error::custom_error;

custom_error! {
    /// Error describing the read syscall
    pub PullError
    WhError{source: WhError} = "{source}",
    NoHostAvailable = "No host available"

    //Theses two errors, for now panic to simplify their detection because they should never happen:
    //PullFolder
    //No Host to hold the file
}

impl NetworkInterface {
    // REVIEW - recheck and simplify this if possible
    pub fn pull_file_sync(&self, file: InodeId) -> Result<Option<Callback>, PullError> {
        let self_addr = LocalConfig::read_lock(&self.local_config, "pull_file_sync")
            .expect("pull_fyle_sync: can't get self_addr")
            .general
            .address
            .clone();
        let arbo = Arbo::n_read_lock(&self.arbo, "pull file sync")?;
        let hosts = {
            if let FsEntry::File(hosts) = &arbo.n_get_inode(file)?.entry {
                hosts
            } else {
                panic!("Pulling a folder is invalid.")
            }
        };

        assert!(hosts.len() != 0, "No hosts hold the file.");

        if hosts.contains(&self_addr) {
            // if the asked file is already on disk
            Ok(None)
        } else {
            let callback = self.callbacks.n_create(Callback::Pull(file))?;
            let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel::<WhResult<()>>();

            // will try to pull on all redundancies until success
            for host in hosts {
                // trying on host `pull_from`
                self.to_network_message_tx
                    .send(ToNetworkMessage::SpecificMessage(
                        (
                            MessageContent::RequestFile(file, self_addr.clone()),
                            Some(status_tx.clone()),
                        ),
                        vec![host.clone()], // NOTE - naive choice for now
                    ))
                    .expect("pull_file: unable to request on the network thread");

                // processing status
                match status_rx
                    .blocking_recv()
                    .expect("pull_file: unable to get status from the network thread")
                {
                    Ok(()) => return Ok(Some(callback)),
                    Err(_) => continue,
                }
            }
            let _ = self.callbacks.resolve(callback, true);
            log::error!("No host is currently able to send the file.\nFile: {file}");
            return Err(PullError::NoHostAvailable);
        }
    }
}
