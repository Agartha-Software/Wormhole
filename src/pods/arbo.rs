use crate::{network::message::Address, providers::whpath::WhPath};
use fuser::FileType;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io, time::Duration};

// SECTION consts

pub const ROOT: InodeId = 0;
pub const LOCK_TIMEOUT: Duration = Duration::new(5, 0);

// !SECTION

// SECTION types

/// InodeId is represented by an u64
pub type Hosts = Vec<Address>;
pub type InodeId = u64;

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Should be extended until meeting [fuser::FileType]
pub enum FsEntry {
    File(Hosts),
    Directory(Vec<InodeId>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Inode {
    pub parent: InodeId,
    pub id: InodeId,
    pub name: String,
    pub entry: FsEntry,
}

pub type ArboIndex = HashMap<InodeId, Inode>;
pub struct Arbo {
    entries: ArboIndex,
}

// !SECTION

// SECTION implementations

impl FsEntry {
    // pub fn get_path(&self) -> &PathBuf {
    //     match self {
    //         FsEntry::File(path) => path,
    //         FsEntry::Directory(children) => path,
    //     }
    // }

    // pub fn get_name(&self) -> io::Result<&OsStr> {
    //     match Path::new(self.get_path()).file_name() {
    //         Some(name) => Ok(name),
    //         None => Err(io::Error::new(io::ErrorKind::Other, "Invalid path ending")),
    //     }
    // }

    pub fn get_filetype(&self) -> FileType {
        match self {
            FsEntry::File(_) => FileType::RegularFile,
            FsEntry::Directory(_) => FileType::Directory,
        }
    }

    pub fn get_children(&self) -> io::Result<&Vec<InodeId>> {
        match self {
            FsEntry::File(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "entry is not a directory",
            )),
            FsEntry::Directory(children) => Ok(children),
        }
    }
}

impl Inode {
    pub fn new(name: String, parent_ino: InodeId, id: InodeId, entry: FsEntry) -> Self {
        Self {
            parent: parent_ino,
            id: id,
            name: name,
            entry: entry,
        }
    }
}

impl Arbo {
    pub fn new() -> Self {
        let mut arbo: Self = Self {
            entries: HashMap::new(),
        };

        arbo.entries.insert(
            ROOT,
            Inode {
                parent: ROOT,
                id: ROOT,
                name: "/".to_owned(),
                entry: FsEntry::Directory(vec![]),
            },
        );

        arbo
    }

    #[must_use]
    pub fn add_inode_from_parameters(
        &mut self,
        name: String,
        ino: InodeId,
        parent_ino: InodeId,
        entry: FsEntry,
    ) -> io::Result<()> {
        if self.entries.contains_key(&ino) {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "file already existing",
            ))
        } else if !self.entries.contains_key(&parent_ino) {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "parent not existing",
            ))
        } else {
            match self.entries.get_mut(&parent_ino) {
                None => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "parent not existing",
                )),
                Some(Inode {
                    parent: _,
                    id: _,
                    name: _,
                    entry: FsEntry::Directory(parent_children),
                }) => {
                    let new_entry = Inode {
                        parent: parent_ino,
                        id: ino,
                        name: name,
                        entry: entry,
                    };
                    parent_children.push(ino);
                    self.entries.insert(ino, new_entry);
                    Ok(())
                }
                Some(_) => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "parent not a folder",
                )),
            }
        }
    }

    #[must_use]
    pub fn add_inode(&mut self, id: InodeId, inode: Inode) -> io::Result<()> {
        self.add_inode_from_parameters(inode.name, id, inode.parent, inode.entry)
    }

    #[must_use]
    pub fn remove_children(&mut self, parent: InodeId, child: InodeId) -> io::Result<()> {
        let parent = self.get_inode_mut(parent)?;

        let children = match &mut parent.entry {
            FsEntry::File(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "remove_children: specified parent is not a folder",
            )),
            FsEntry::Directory(children) => Ok(children),
        }?;

        children.retain(|v| *v != child);
        Ok(())
    }

    #[must_use]
    pub fn remove_inode(&mut self, id: InodeId) -> io::Result<Inode> {
        let removed = match self.entries.remove(&id) {
            Some(inode) => Ok(inode),
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "remove_inode: specified inode not found",
            )),
        }?;

        self.remove_children(removed.parent, id)?;

        Ok(removed)
    }

    #[must_use]
    pub fn get_inode(&self, ino: InodeId) -> io::Result<&Inode> {
        match self.entries.get(&ino) {
            Some(inode) => Ok(inode),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "entry not found")),
        }
    }

    // not public as the modifications are not automaticly propagated on other related inodes
    #[must_use]
    fn get_inode_mut(&mut self, ino: InodeId) -> io::Result<&mut Inode> {
        match self.entries.get_mut(&ino) {
            Some(inode) => Ok(inode),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "entry not found")),
        }
    }

    #[must_use]
    pub fn get_path_from_inode_id(&self, inode_index: InodeId) -> io::Result<WhPath> {
        if inode_index == ROOT {
            return Ok(WhPath::from("/"));
        }
        let inode = match self.entries.get(&inode_index) {
            Some(inode) => inode,
            None => {
                return Err(io::Error::new(io::ErrorKind::NotFound, "entry not found"));
            }
        };

        let mut parent_path = self.get_path_from_inode_id(inode.parent)?;
        parent_path.push(&inode.name.clone());
        Ok(parent_path)
    }

    #[must_use]
    pub fn get_inode_child_by_name(&self, parent: &Inode, name: &String) -> io::Result<&Inode> {
        if let Ok(children) = parent.entry.get_children() {
            for child in children.iter() {
                if let Some(child) = self.entries.get(child) {
                    if child.name == *name {
                        return Ok(child);
                    }
                }
            }
            Err(io::Error::new(io::ErrorKind::NotFound, "entry not found"))
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "entry is not a directory",
            ))
        }
    }

    #[must_use]
    pub fn get_inode_from_path(&self, mut path: WhPath) -> io::Result<&Inode> {
        let mut actual_inode = self.entries.get(&ROOT).expect("inode_from_path: NO ROOT");

        for name in path.to_vector().iter() {
            actual_inode = self.get_inode_child_by_name(&actual_inode, name)?;
        }

        Ok(actual_inode)
    }
}

// !SECTION

fn index_folder_recursive(
    arbo: &mut Arbo,
    inode: &mut u64,
    path: &WhPath,
) -> io::Result<()> {
    let errors_nb = root_fd
        .list_dir(&path)?
        .map(|entry| -> io::Result<()> {
            let entry = entry?;

            let name = entry.file_name();
            let stype = entry.simple_type().unwrap();

            let generated_path = path.join(name);

            let new_entry = match stype {
                SimpleType::Dir => FsEntry::Directory(generated_path.clone()),
                SimpleType::File => FsEntry::File(generated_path.clone(), vec![]),
                _ => return Ok(()),
            };
            arbo.insert(*inode, new_entry);
            println!("added entry to arbo {}:{:?}", inode, arbo.get(inode));
            *inode += 1;

            if stype == SimpleType::Dir {
                index_folder_recursive(arbo, inode, root_fd, generated_path)?;
            }
            Ok(())
        })
        .filter(|e| e.is_err())
        .collect::<Vec<Result<(), io::Error>>>()
        .len();
    println!(
        "indexing: {} error(s) in folder {}",
        errors_nb,
        path.display()
    );
    Ok(())
}

pub fn index_folder(path: &WhPath) -> io::Result<Arbo> {
    let mut arbo = Arbo::new();
    let mut inode: u64 = 2;

    arbo.add_inode(
        1,
        Inode::new("".to_owned(), 1, 1, FsEntry::Directory(Vec::new())),
    );
    index_folder_recursive(&mut arbo, &mut inode, path)?;
    Ok(arbo)
}