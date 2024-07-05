use serde::{Deserialize, Serialize};

use crate::data::metadata::MetaData;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkMessage {
    File(File),
    Meta(MetaData),
    NewFolder(Folder),
    /// old, new
    Rename(std::path::PathBuf, std::path::PathBuf),
    RequestFile(std::path::PathBuf),
    Binary(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub path: std::path::PathBuf,
    pub file: Vec<u8>,
    pub ino: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Folder {
    pub ino: u64,
    pub path: std::path::PathBuf,
}
