use std::{fs, path::Path, str, sync::Arc};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    error::{WhError, WhResult},
    pods::arbo::LOCK_TIMEOUT,
};

/** NOTE
 * To add elements in the configuration file :
 * To create a superior field like [field], create a new structure and add it to the Metadata struct
 * Minors fields are named in the structure you added to Metadata
 * the section name is the same as the name of the value of your new struct in Metadata
 */

pub trait Config: Serialize + DeserializeOwned {
    fn write<S: AsRef<Path>>(&self, path: S) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = toml::to_string(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    fn read<S: AsRef<Path>>(path: S) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    #[must_use]
    fn read_lock<'a, T: Config>(
        conf: &'a Arc<RwLock<T>>,
        called_from: &'a str,
    ) -> WhResult<RwLockReadGuard<'a, T>> {
        conf.try_read_for(LOCK_TIMEOUT).ok_or(WhError::WouldBlock {
            called_from: called_from.to_owned(),
        })
    }

    #[must_use]
    fn write_lock<'a, T: Config>(
        conf: &'a Arc<RwLock<T>>,
        called_from: &'a str,
    ) -> WhResult<RwLockWriteGuard<'a, T>> {
        conf.try_write_for(LOCK_TIMEOUT).ok_or(WhError::WouldBlock {
            called_from: called_from.to_owned(),
        })
    }
}

impl<T: Serialize + DeserializeOwned> Config for T {}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GlobalConfig {
    pub general: GeneralGlobalConfig,
    pub redundancy: RedundancyConfig,
}

impl GlobalConfig {
    pub fn add_hosts(
        mut self,
        url: Option<String>,
        mut additional_hosts: Vec<String>,
    ) -> GlobalConfig {
        if let Some(url) = url {
            additional_hosts.insert(0, url);
        }

        self.general.entrypoints.extend(additional_hosts);
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeneralGlobalConfig {
    /// name of the network
    pub name: String,
    /// network urls to join the netwoek from
    pub entrypoints: Vec<String>,
    /// hostnames of known peers
    pub hosts: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RedundancyConfig {
    pub number: u64,
}

impl Default for RedundancyConfig {
    fn default() -> Self {
        Self { number: 2 }
    }
}
