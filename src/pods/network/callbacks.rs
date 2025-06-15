use std::{collections::HashMap, io};

use parking_lot::RwLock;
use tokio::sync::broadcast;

use crate::{
    error::{WhError, WhResult},
    pods::arbo::{InodeId, LOCK_TIMEOUT},
};

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum Callback {
    Pull(InodeId),
    PullFs,
}

#[derive(Debug)]
pub struct Callbacks {
    pub callbacks: RwLock<HashMap<Callback, broadcast::Sender<bool>>>,
}

impl Callbacks {
    pub fn n_create(&self, call: Callback) -> WhResult<Callback> {
        if let Some(mut callbacks) = self.callbacks.try_write_for(LOCK_TIMEOUT) {
            if !callbacks.contains_key(&call) {
                let (tx, _) = broadcast::channel(1);

                callbacks.insert(call, tx);
            };
            Ok(call)
        } else {
            Err(crate::error::WhError::WouldBlock {
                called_from: "create callback".to_owned(),
            })
        }
    }

    pub fn create(&self, call: Callback) -> io::Result<Callback> {
        if let Some(mut callbacks) = self.callbacks.try_write_for(LOCK_TIMEOUT) {
            if !callbacks.contains_key(&call) {
                let (tx, _) = broadcast::channel(1);

                callbacks.insert(call, tx);
            };
            Ok(call)
        } else {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "unable to write_lock callbacks",
            ))
        }
    }

    pub fn resolve(&self, call: Callback, status: bool) -> io::Result<()> {
        if let Some(mut callbacks) = self.callbacks.try_write_for(LOCK_TIMEOUT) {
            if let Some(cb) = callbacks.remove(&call) {
                cb.send(status).map(|_| ()).map_err(|send_error| {
                    io::Error::new(io::ErrorKind::AddrNotAvailable, send_error.to_string())
                })
            } else {
                Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "no such callback active",
                ))
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "unable to read_lock callbacks",
            ))
        }
    }

    pub fn n_wait_for(&self, call: Callback) -> WhResult<bool> {
        let mut waiter = if let Some(callbacks) = self.callbacks.try_read_for(LOCK_TIMEOUT) {
            if let Some(cb) = callbacks.get(&call) {
                cb.subscribe()
            } else {
                return Err(WhError::WouldBlock {
                    called_from: "no such callback active".to_string(),
                });
            }
        } else {
            return Err(WhError::WouldBlock {
                called_from: "unable to read_lock callbacks".to_string(),
            });
        };

        match waiter.blocking_recv() {
            Ok(status) => Ok(status),
            Err(_) => Ok(false), // maybe change to a better handling
        }
    }

    pub fn wait_for(&self, call: Callback) -> io::Result<bool> {
        let mut waiter = if let Some(callbacks) = self.callbacks.try_read_for(LOCK_TIMEOUT) {
            if let Some(cb) = callbacks.get(&call) {
                cb.subscribe()
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "no such callback active",
                ));
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "unable to read_lock callbacks",
            ));
        };

        match waiter.blocking_recv() {
            Ok(status) => Ok(status),
            Err(_) => Ok(false), // maybe change to a better handling
        }
    }

    pub async fn async_wait_for(&self, call: Callback) -> io::Result<bool> {
        let mut waiter = if let Some(callbacks) = self.callbacks.try_read_for(LOCK_TIMEOUT) {
            if let Some(cb) = callbacks.get(&call) {
                cb.subscribe()
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "no such callback active",
                ));
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "unable to read_lock callbacks",
            ));
        };

        match waiter.recv().await {
            Ok(status) => Ok(status),
            Err(_) => Ok(false), // maybe change to a better handling
        }
    }
}
