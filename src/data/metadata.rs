
use serde::{Deserialize, Serialize};
use std::os::unix::fs::MetadataExt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaData {
    name: String,
    // checksum: Sha256,
    mtime: std::time::SystemTime,
    size: u64,
    owners: Vec<String>,
}


impl MetaData {
    pub fn read(path: &String) -> Result<Self, Box<dyn std::error::Error>> {
        let stat =  std::fs::metadata(path)?;
        Ok(Self {
            name: path.clone(),
            // checksum: Sha256::new().input(file),
            size: stat.size(),
            owners: vec!(),
            mtime: stat.modified()?,
        })
    }
}
