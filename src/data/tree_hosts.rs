use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{network::message::Address, pods::itree::InodeId};

pub type TreeLine = (u8, InodeId, String, Vec<Address>); // (indentation_level, ino, path, hosts)

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CliHostTree {
    pub lines: Vec<TreeLine>,
}

impl fmt::Display for CliHostTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::new();
        for (indent, ino, path, hosts) in &self.lines {
            output.push_str(&format!(
                "{}[{ino}] {:?}    ->    ({}) {:?}\n",
                generate_indentation(*indent),
                path,
                hosts.len(),
                hosts
            ));
        }
        write!(f, "{output}")
    }
}

fn generate_indentation(n: u8) -> String {
    let result = " |  ".to_string();
    result.repeat(n.into())
}
