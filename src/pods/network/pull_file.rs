use std::io;
use std::sync::Arc;

use crate::network::message::{MessageContent, ToNetworkMessage};
use crate::pods::itree::{FsEntry, ITree};
use crate::pods::network::callbacks::Request;
use crate::pods::network::network_interface::NetworkInterface;
use crate::{error::WhError, pods::itree::InodeId};
use custom_error::custom_error;
use tokio::sync::mpsc;

custom_error! {
    #[derive(Clone)]
    /// Error describing the read syscall
    pub PullError
    WhError{source: WhError} = "{source}",
    NoHostAvailable = "No host available",

    // Arc required to keep clonability
    WriteError{io: Arc<io::Error>} = "failed to write: {io}",

    //Theses two errors, for now panic to simplify their detection because they should never happen:
    //PullFolder
    //No Host to hold the file
}

impl NetworkInterface {
    /// Pull the file from the network
    /// Returns a copy of the file's buffer if it was pulled
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    pub fn pull_file_sync(&self, file: InodeId) -> Result<Option<Arc<Vec<u8>>>, PullError> {
        let itree = ITree::read_lock(&self.itree, "pull file sync")?;
        let hosts = {
            if let FsEntry::File(hosts) = &itree.get_inode(file)?.entry {
                hosts
            } else {
                return Err(WhError::InodeIsADirectory.into());
            }
        };

        if hosts.is_empty() {
            return Err(PullError::NoHostAvailable);
        }

        let hostname = self.hostname()?;

        if hosts.contains(&hostname) {
            // if the asked file is already on disk
            Ok(None)
        } else {
            let procedure = || {
                // will try to pull on all redundancies until success
                for host in hosts {
                    let (tx, mut rx) = mpsc::unbounded_channel();
                    // trying on host `pull_from`
                    self.to_network_message_tx
                        .send(ToNetworkMessage::SpecificMessage(
                            (MessageContent::RequestFile(file), Some(tx)),
                            vec![host.clone()], // NOTE - naive choice for now
                        ))
                        .expect("pull_file: unable to request on the network thread");

                    // processing status
                    match rx.blocking_recv() {
                        Some(_) => return Ok(()),
                        _ => continue,
                    }
                }
                log::error!("No host is currently able to send the file.\nFile: {file}");
                Err(PullError::NoHostAvailable)
            };

            self.callbacks.request_sync(&Request::Pull(file), procedure)
        }
    }
}
