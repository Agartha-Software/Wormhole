use std::fmt::{self, Debug};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::pods::{
    itree::{FsEntry, Ino, Inode},
    whpath::WhPath,
};

pub type TreeLine = (u8, Ino, WhPath, FsEntry); // (indentation_level, ino, path, hosts)

#[derive(Clone, Serialize, Deserialize, TS)]
pub enum TreeEntry {
    Directory(Inode, Vec<TreeEntry>),
    File(Inode),
}
#[derive(Serialize, Deserialize, TS)]
pub struct TreeData {
    pub tree: TreeEntry,
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
                let ino = &inode.id;
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
                let ino = &inode.id;
                let data = match &inode.entry {
                    FsEntry::File(hosts) => format!(" : {hosts:?}"),
                    FsEntry::Symlink(symlink) => format!(" -> {}", symlink.target),
                    // should never happen, but is a sane fallback:
                    FsEntry::Directory(_) => "".to_owned(),
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

#[cfg(test)]
mod test {
    use crate::{
        data::tree_hosts::TreeEntry,
        pods::{
            itree::{EntrySymlink, FsEntry, Inode},
            whpath::InodeName,
        },
    };

    #[test]
    pub fn test_formatting() {
        let root = Inode::new(
            InodeName::try_from("".to_owned()).expect("\"\" is a valid InodeName"),
            1,
            1,
            FsEntry::new_directory(),
            0,
        );
        let folder = Inode::new(
            InodeName::try_from("folder".to_owned()).expect("\"folder\" is a valid InodeName"),
            1,
            10,
            FsEntry::new_directory(),
            0,
        );
        let file = Inode::new(
            InodeName::try_from("file".to_owned()).expect("\"file\" is a valid InodeName"),
            10,
            11,
            FsEntry::new_file(),
            0,
        );
        let empty = Inode::new(
            InodeName::try_from("empty".to_owned()).expect("\"empty\" is a valid InodeName"),
            10,
            12,
            FsEntry::new_directory(),
            0,
        );
        let link = Inode::new(
            InodeName::try_from("link".to_owned()).expect("\"link\" is a valid InodeName"),
            10,
            13,
            FsEntry::Symlink(
                EntrySymlink::parse("/mountpoint/folder/file", "/mountpoint")
                    .expect("this symlink is valid"),
            ),
            0,
        );

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
