use std::collections::HashMap;

use crate::{
    error::WhResult,
    pods::{itree::ITree, pod::Pod},
};

impl Pod {
    /// Returns a hasmap of all found file extensions, and the number of found files for each
    pub fn get_stats_per_filetype(&self) -> WhResult<HashMap<String, u64>> {
        let itree = ITree::read_lock(&self.network_interface.itree, "Pod::get_stats_per_filetype")?;

        Ok(itree.get_file_tree_and_hosts(None)?.into_iter().fold(
            HashMap::new(),
            |mut acc, (_, _, path, _)| {
                let filetype = path.rsplit_once(".").map_or("unknown", |s| s.1);
                if let Some(entry) = acc.get(filetype) {
                    acc.insert(filetype.to_string(), entry + 1)
                } else {
                    acc.insert(filetype.to_string(), 1)
                };
                acc
            },
        ))
    }
}
