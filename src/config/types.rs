use std::fs;

use serde::{Deserialize, Serialize};

/** NOTE
 * To add elements in the configuration file :
 * To create a superior field like [field], create a new structure and add it to the Metadata struct
 * Minors fields are named in the structure you added to Metadata
 * the section name is the same as the name of the value of your new struct in Metadata
 */

pub trait Config : Sized {
    #[must_use]
    fn write<S:  AsRef<std::path::Path>>(&self, path: S) -> Result<(), Box<dyn std::error::Error>>;
    #[must_use]
    fn read<S: AsRef<std::path::Path>>(path: S) -> Result<Self, Box<dyn std::error::Error>>;
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    essential: EssentialConfig,
    optional: Option<OptionalConfig>,
}

#[derive(Debug, Deserialize)]
pub struct EssentialConfig {
    name: String,
    ip: String,
}

#[derive(Debug, Deserialize)]
pub struct OptionalConfig {
    redundancy: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Network {
    pub name: String,
    pub peers: Vec<String>,
}

impl Network {
    pub fn new(peers: Vec<String>, name: String) -> Self {
        Self {
            name,
            peers,
        }
    }
}

impl Config for Network {
    #[must_use]
     fn write<S: AsRef<std::path::Path>>(&self, path: S) -> Result<(), Box<dyn std::error::Error>> {
        let mut serialized = toml::to_string(&self)?;
        serialized.insert_str(0, "# This file is automatically generated and replicated\n# Modifying it will lead to errors\n");
        fs::write(&path, &serialized)?;
        Ok(())
    }

    fn read<S: AsRef<std::path::Path>>(path: S) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let deserialized = toml::from_str::<Self>(&contents)?;
        Ok(deserialized)
    }
}