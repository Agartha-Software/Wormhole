use std::{collections::HashMap, fmt::Debug, future::Future, sync::Arc};

use parking_lot::RwLock;
use tokio::sync::broadcast;

use crate::{
    error::{WhError, WhResult},
    pods::{
        itree::{InodeId, LOCK_TIMEOUT},
        network::pull_file::PullError,
    },
};

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum Request {
    GetSignature(InodeId, String),
    Pull(InodeId),
    PullFs,
}

type Shot<File = Arc<Vec<u8>>> = Result<Option<File>, PullError>;

///
/// Deduplicate costly network actions by letting only one run at a time
#[derive(Debug)]
pub struct Callbacks {
    callbacks: RwLock<HashMap<Request, broadcast::Sender<Shot>>>,
}

impl Callbacks {
    ///
    /// create a waiting callback or wait for existing
    ///
    pub async fn request<R: Future, F: FnOnce() -> R>(&self, call: &Request, procedure: F) -> Shot {
        let mut waiter = if let Some(mut callbacks) = self.callbacks.try_write_for(LOCK_TIMEOUT) {
            if let Some(cb) = callbacks.get(call) {
                // safety: the sender is removed before it is pushed.
                // If the sender is present, no content has been sent, and none will be sent until the lock is released
                cb.subscribe()
            } else {
                let (tx, rx) = broadcast::channel(1);
                callbacks.insert(call.clone(), tx);
                rx
            }
        } else {
            return Err(WhError::WouldBlock {
                called_from: "unable to write_lock callbacks".to_string(),
            }
            .into());
        };
        procedure().await;

        match waiter.recv().await {
            Ok(s) => s,
            Err(_) => Err(WhError::WouldBlock {
                called_from: "callback dropped".to_owned(),
            }
            .into()),
        }
    }

    ///
    /// create a waiting callback or wait for existing
    ///
    /// # Panics
    ///
    /// This function panics if called within an asynchronous execution
    /// context.
    ///
    pub fn request_sync<F, E>(&self, call: &Request, procedure: F) -> Shot
    where
        PullError: From<E>,
        F: FnOnce() -> Result<(), E>,
    {
        let (need_procedure, mut waiter) =
            if let Some(mut callbacks) = self.callbacks.try_write_for(LOCK_TIMEOUT) {
                if let Some(cb) = callbacks.get(call) {
                    // safety: the sender is removed before it is pushed.
                    // If the sender is present, no content has been sent, and none will be sent until the lock is released
                    (false, cb.subscribe())
                } else {
                    let (tx, rx) = broadcast::channel(1);
                    callbacks.insert(call.clone(), tx);
                    (true, rx)
                }
            } else {
                return Err(WhError::WouldBlock {
                    called_from: "unable to read_lock callbacks".to_string(),
                }
                .into());
            };

        if need_procedure {
            procedure()?;
        }

        match waiter.blocking_recv() {
            Ok(s) => s,
            Err(_) => Err(WhError::WouldBlock {
                called_from: "callback dropped".to_owned(),
            }
            .into()),
        }
    }

    /// resolve a callback and resume waiting threads
    /// removes the callback from the queue
    pub fn resolve(&self, call: Request, answer: Shot) -> WhResult<()> {
        if let Some(mut callbacks) = self.callbacks.try_write_for(LOCK_TIMEOUT) {
            if let Some(cb) = callbacks.remove(&call) {
                // safety: removing the sender before fulfilling it ensures no one subscribes to it after it's been fulfilled
                let _ = cb.send(answer); // we don't care if the send fails because it only means no one's waiting
                Ok(())
            } else {
                Err(WhError::InodeNotFound) // TODO: this is a 'not found' error, not specifically of an inode not found
            }
        } else {
            Err(WhError::WouldBlock {
                called_from: "unable to read_lock callbacks".into(),
            })
        }
    }

    pub fn new() -> Self {
        Self {
            callbacks: RwLock::new(HashMap::new()),
        }
    }
}
