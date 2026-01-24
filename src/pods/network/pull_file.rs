use std::io;
use std::sync::Arc;

use crate::network::message::{Request, Response, ToNetworkMessage};
use crate::pods::itree::{FsEntry, ITree};
use crate::pods::network::network_interface::NetworkInterface;
use crate::{error::WhError, pods::itree::Ino};
use custom_error::custom_error;
use tokio::sync::oneshot;

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
    /// Doesn't affect the itree or the disk
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    pub fn pull_file(&self, ino: Ino) -> Result<Option<Vec<u8>>, PullError> {
        let hosts = {
            let itree = ITree::read_lock(&self.itree, "pull file sync")?;

            if let FsEntry::File(hosts) = &itree.get_inode(ino)?.entry {
                hosts.clone()
            } else {
                return Err(WhError::InodeIsADirectory.into());
            }
        };

        if hosts.is_empty() {
            return Err(PullError::NoHostAvailable);
        }

        if hosts.contains(&self.id) {
            // if the asked file is already on disk
            Ok(None)
        } else {
            let mut file = None;

            // will try to pull on all redundancies until success
            for host in hosts {
                let (tx, rx) = oneshot::channel();
                // trying on host `pull_from`
                self.to_network_message_tx
                    .send(ToNetworkMessage::AnswerMessage(
                        Request::RequestFile(ino),
                        tx,
                        host, // NOTE - naive choice for now
                    ))
                    .expect("pull_file: unable to request on the network thread");

                // processing status
                match rx.blocking_recv() {
                    Ok(Some(Response::RequestedFile(request))) => file = Some(request),
                    Ok(Some(_)) => panic!("Wrong Reponse received!"),
                    _ => continue,
                };
            }

            match file {
                Some(data) => Ok(Some(data)),
                None => {
                    log::error!("No host is currently able to send the file.\nFile: {ino}");
                    Err(PullError::NoHostAvailable)
                }
            }
        }
    }
}
