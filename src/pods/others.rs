use std::collections::HashMap;

use crate::{
    error::WhResult,
    pods::{filesystem::fs_interface::SimpleFileType, itree::ITree, pod::Pod},
};

impl Pod {
    /// Returns a hasmap of all found file extensions, and the number of found files for each
    pub fn get_stats_per_filetype(&self) -> WhResult<HashMap<String, u64>> {
        let itree = ITree::read_lock(&self.network_interface.itree, "Pod::get_stats_per_filetype")?;

        Ok(itree.iter().fold(HashMap::new(), |mut acc, (_, inode)| {
            if inode.entry.get_filetype() == SimpleFileType::File {
                let filetype = inode
                    .name
                    .as_str()
                    .rsplit_once(".")
                    .map_or("unknown", |s| s.1);
                *acc.entry(filetype.to_owned()).or_default() += 1;
            }
            acc
        }))
    }
}
