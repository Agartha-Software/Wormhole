use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaData {
    name: std::path::PathBuf,
    // checksum: Sha256,
    mtime: std::time::SystemTime,
    size: u64,
    owners: Vec<bool>,
}

impl MetaData {
    pub fn read(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let stat = std::fs::metadata(path)?;
        #[cfg(target_os = "windows")]
        let size = stat.file_size(); 
        #[cfg(target_os = "linux")]
        let size = stat.size(); 
        Ok(Self {
            name: path.to_path_buf(),
            // checksum: Sha256::new().input(file),
            size,
            owners: vec![],
            mtime: stat.modified()?,
        })
    }
}
