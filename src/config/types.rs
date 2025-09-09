use std::{fs, path::Path, str, sync::Arc};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    error::{CliError, WhError, WhResult},
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
pub struct LocalConfig {
    pub general: GeneralLocalConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeneralLocalConfig {
    // pub name: String,
    pub hostname: String,
    pub url: Option<String>,
}

impl LocalConfig {
    pub fn constructor(&mut self, local: Self) -> Result<(), CliError> {
        // self.general.name = local.general.name;
        if local.general.hostname != self.general.hostname {
            log::warn!("Local Config: Impossible to modify an ip address");
            return Err(CliError::Unimplemented {
                arg: "Local Config: Impossible to modify an ip address".to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GlobalConfig {
    pub general: GeneralGlobalConfig,
    pub redundancy: RedundancyConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeneralGlobalConfig {
    /// name of the network
    pub name: String,
    /// network urls to join the netwoek from
    pub entrypoints: Vec<String>,
    /// hostnames of known peers
    pub hosts: Vec<String>,
    /// paths not tracked by wormhole
    pub ignore_paths: Vec<String>, //FIXME - What is this ???
    /// ?
    pub pods_names: Vec<String>,
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

impl GlobalConfig {
    // pub fn constructor(&mut self, global: Self) -> Result<(), CliError> {
    //     self.general.ignore_paths = global.general.ignore_paths;
    //     self.general.pods_names = global.general.pods_names;
    //     if global.general.entrypoints != self.general.entrypoints {
    //         log::warn!("Global Config: Impossible to modify peers' ip address");
    //         return Err(CliError::Unimplemented {
    //             arg: "Global Config: Impossible to modify peers' ip address".to_owned(),
    //         });
    //     }
    //     self.redundancy.number = global.redundancy.number;

    //     Ok(())
    // }
}

//OLD
//OLD
//OLD
//OLD

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Network {
    pub name: String,
    pub peers: Vec<String>,
}

impl Network {
    pub fn new(peers: Vec<String>, name: String) -> Self {
        Self { name, peers }
    }
}
