use std::{
    collections::HashMap,
    fmt::{self, Debug},
    io,
};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::WhError,
    ipc::answers::TreeAnswer,
    pods::{
        itree::{EntrySymlink, FsEntry, ITreeIndex, Ino, Inode, ROOT},
        pod::Pod,
        whpath::WhPath,
    },
};

#[derive(Clone, Serialize, Deserialize, TS)]
enum FsEntryInfo {
    Directory,
    Symlink(EntrySymlink),
    File(Vec<String>),
}

impl FsEntryInfo {
    fn from(value: FsEntry, infos: &HashMap<PeerId, String>) -> Self {
        match value {
            FsEntry::File(peer_ids) => Self::File(
                peer_ids
                    .iter()
                    .map(|s| {
                        infos
                            .get(s)
                            .cloned()
                            .unwrap_or_else(||s.to_base58())
                    })
                    .collect(),
            ),
            FsEntry::Directory(_) => Self::Directory,
            FsEntry::Symlink(symlink) => Self::Symlink(symlink.clone()),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, TS)]
struct InodeInfo {
    name: String,
    ino: Ino,
    entry: FsEntryInfo,
}

impl InodeInfo {
    fn from(value: Inode, infos: &HashMap<PeerId, String>) -> Self {
        Self {
            name: value.name.to_string(),
            ino: value.id,
            entry: FsEntryInfo::from(value.entry, infos),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, TS)]
enum TreeEntry {
    Directory(InodeInfo, Vec<TreeEntry>),
    File(InodeInfo),
}
#[derive(Serialize, Deserialize, TS)]
pub struct TreeData {
    tree: TreeEntry,
}

impl Debug for TreeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tree.fmt(f)
    }
}

impl fmt::Debug for TreeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct PadAdapterState {
            on_newline: bool,
        }

        impl PadAdapterState {
            /// Behavior is different to std's because we already add the "├── " padding
            /// we need it to only indent 'on the second block'
            fn default() -> Self {
                PadAdapterState { on_newline: false }
            }
        }
        struct PadAdapter<'buf, 'fmt, 'state> {
            buf: &'buf mut fmt::Formatter<'fmt>,
            state: &'state mut PadAdapterState,
            string: String,
        }

        impl<'buf, 'fmt, 'state> PadAdapter<'buf, 'fmt, 'state> {
            fn wrap(
                fmt: &'buf mut fmt::Formatter<'fmt>,
                state: &'state mut PadAdapterState,
                string: String,
            ) -> PadAdapter<'buf, 'fmt, 'state> {
                PadAdapter::<'buf, 'fmt, 'state> {
                    buf: fmt,
                    state,
                    string,
                }
            }
        }

        impl fmt::Write for PadAdapter<'_, '_, '_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                for s in s.split_inclusive('\n') {
                    if self.state.on_newline {
                        self.buf.write_str(&self.string)?;
                    }

                    self.state.on_newline = s.ends_with('\n');
                    self.buf.write_str(s)?;
                }

                Ok(())
            }

            fn write_char(&mut self, c: char) -> fmt::Result {
                if self.state.on_newline {
                    self.buf.write_str(&self.string)?;
                }
                self.state.on_newline = c == '\n';
                self.buf.write_char(c)
            }
        }

        use fmt::Write;

        match self {
            TreeEntry::Directory(inode, items) => {
                let name = &inode.name;
                let ino = &inode.ino;
                write!(f, "{name} ({ino})")?;
                if !items.is_empty() {
                    f.write_char('\n')?;
                }

                for item in items[..items.len().saturating_sub(1)].iter() {
                    let mut state = PadAdapterState::default();
                    let mut padded_f = PadAdapter::wrap(f, &mut state, "│   ".to_owned());
                    padded_f.write_str("├── ")?;

                    padded_f.write_fmt(format_args!("{item:?}\n"))?;
                }
                if let Some(last) = items.last() {
                    let mut state = PadAdapterState::default();
                    let mut padded_f = PadAdapter::wrap(f, &mut state, "    ".to_owned());
                    padded_f.write_str("└── ")?;
                    padded_f.write_fmt(format_args!("{last:?}"))?;
                }
                Ok(())
            }
            TreeEntry::File(inode) => {
                let name = &inode.name;
                let ino = &inode.ino;
                let data = match &inode.entry {
                    FsEntryInfo::File(hosts) => format!(" : {hosts:?}"),
                    FsEntryInfo::Symlink(symlink) => format!(" -> {}", symlink.target),
                    // should never happen, but is a sane fallback:
                    FsEntryInfo::Directory => "".to_owned(),
                };
                write!(f, "{name} ({ino}){data}")
            }
        }
    }
}

impl fmt::Display for TreeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.tree.fmt(f)
    }
}

fn recurse_build_tree(
    itree: &mut ITreeIndex,
    ino: Ino,
    infos: &HashMap<PeerId, String>,
) -> Option<TreeEntry> {
    if let Some(inode) = itree.remove(&ino) {
        match &inode.entry {
            crate::pods::itree::FsEntry::Directory(children) => {
                let children = children
                    .iter()
                    .flat_map(|ino| recurse_build_tree(itree, *ino, infos))
                    .collect();
                Some(TreeEntry::Directory(
                    InodeInfo::from(inode, infos),
                    children,
                ))
            }
            _ => Some(TreeEntry::File(InodeInfo::from(inode, infos))),
        }
    } else {
        None
    }
}

pub fn get_tree(pod: &Pod, path: Option<&WhPath>) -> TreeAnswer {
    let infos = HashMap::from_iter(
        pod.network_interface
            .peers_info
            .read()
            .iter()
            .map(|(k, v)| (*k, v.nickname.clone()))
            .chain([(pod.network_interface.id, pod.nickname.clone())]),
    );
    let itree = pod.fs_interface.network_interface.itree.read();
    let start = path
        .map(|p| itree.get_inode_from_path(p).map(|inode| inode.id))
        .unwrap_or(Ok(ROOT));

    let mut itree = {
        let owned = itree.raw_entries().clone();
        drop(itree);
        owned
    };

    let tree = start.and_then(|start| {
        recurse_build_tree(&mut itree, start, &infos).ok_or(WhError::InodeNotFound)
    });

    match tree {
        Ok(tree) => TreeAnswer::Tree(Box::new(TreeData { tree })),
        Err(err) => {
            log::error!("Failed in an unexpected way when getting tree: {err}");
            TreeAnswer::PodTreeFailed(io::Error::other(err).into())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        data::tree_hosts::{FsEntryInfo, InodeInfo, TreeEntry},
        pods::itree::{EntrySymlink, ROOT},
    };

    #[test]
    pub fn test_formatting() {
        let root = InodeInfo {
            name: "/".to_owned(),
            ino: ROOT,
            entry: FsEntryInfo::Directory,
        };
        let folder = InodeInfo {
            name: "folder".to_owned(),
            ino: 10,
            entry: FsEntryInfo::Directory,
        };
        let file = InodeInfo {
            name: "file".to_owned(),
            ino: 11,
            entry: FsEntryInfo::File(vec![]),
        };
        let empty = InodeInfo {
            name: "empty".to_owned(),
            ino: 12,
            entry: FsEntryInfo::Directory,
        };
        let link = InodeInfo {
            name: "link".to_owned(),
            ino: 13,
            entry: FsEntryInfo::Symlink(
                EntrySymlink::parse("/mountpoint/folder/file", "/mountpoint")
                    .expect("this symlink is valid"),
            ),
        };

        let t_file = TreeEntry::File(file.clone());
        let t_link = TreeEntry::File(link.clone());
        let t_empty = TreeEntry::Directory(empty.clone(), [].to_vec());
        let t_folder = TreeEntry::Directory(
            folder.clone(),
            [t_file.clone(), t_empty.clone(), t_file.clone(), t_link].to_vec(),
        );
        let t_root = TreeEntry::Directory(root, [t_folder].to_vec());
        // println!("{t_root:?}");
        let expected = "\
/ (1)
└── folder (10)
    ├── file (11) : []
    ├── empty (12)
    ├── file (11) : []
    └── link (13) -> //folder/file";
        assert_eq!(&format!("{t_root:#?}"), expected);
    }
}
