use serde::{Deserialize, Serialize};

use crate::data::metadata::MetaData;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetworkMessage {
    File(File),
    Meta(MetaData),
    NewFolder,
    RequestFile(Path),
    Binary(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct File {
    pub pod: String,
    pub path: String,
    pub file: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Path {
    pub pod: String,
    pub file: String,
}
