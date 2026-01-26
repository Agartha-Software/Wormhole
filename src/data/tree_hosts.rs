use std::fmt;

use crate::pods::{
    itree::{FsEntry, Ino},
    whpath::WhPath,
};

pub type TreeLine = (u8, Ino, WhPath, FsEntry); // (indentation_level, ino, path, hosts)
pub struct CliHostTree {
    pub lines: Vec<TreeLine>,
}

impl fmt::Display for CliHostTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::new();
        for (indent, ino, path, entry) in &self.lines {
            let ident = generate_indentation(*indent);
            match entry {
                FsEntry::File(hosts) => {
                    let len = hosts.len();
                    output.push_str(&format!(
                        "{ident}[{ino}] {path:?}    ->    ({len}) {hosts:?}\n",
                    ));
                }
                FsEntry::Directory(_) => {}
                FsEntry::Symlink(symlink) => {
                    let target = &symlink.target;
                    output.push_str(&format!("{ident}[{ino}] {path:?}    ->    {target}\n",));
                }
            }
        }
        write!(f, "{output}")
    }
}

fn generate_indentation(n: u8) -> String {
    let result = " |  ";
    result.repeat(n.into())
}
