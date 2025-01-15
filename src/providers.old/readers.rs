/*
 * Used for the OUTSIDE interactions
 * (actually reading mirror folder, but network one day)
 */

use fuser::FileAttr;
use log::debug;
use log::info;
use std::ffi::OsStr;
use std::io;
use std::io::Read;

use super::FsEntry;
use super::{InodeIndex, Provider, TEMPLATE_FILE_ATTR};

// should maybe be restructured, but
// those are the functions made freely by us following our needs
// and they are directly used by the fuse lib
impl Provider {
    // Used directly in the FuseControler read function
    pub fn read(&self, ino: InodeIndex) -> io::Result<Vec<u8>> {
        println!("read called on ino {}", ino); // DEBUG
        match self.mirror_path_from_inode(ino) {
            Ok(path) => {
                info!("mirror path from inode is {:?}", path);
                let mut file = self.metal_handle.open_file(&path)?;
                let mut buf: Vec<u8> = vec![];
                file.read_to_end(&mut buf)?;
                Ok(buf)
            }
            Err(e) => Err(e),
        }
    }

    // list files inodes in the parent folder
    // List from hashmap and not from disk
    fn list_files(&self, parent_ino: InodeIndex) -> io::Result<Vec<u64>> {
        println!("list files called on ino {}", parent_ino); // DEBUG
        match self.index.get(&parent_ino) {
            Some(entry) => {
                let parent_path = entry.get_path();

                debug!("LISTING files in parent path {:?}", parent_path);
                let ino_list = self
                    .index
                    .iter()
                    .filter_map(|(ino, entry)| {
                        let e_path = entry.get_path();
                        let e_parent = e_path.parent()?;

                        if e_path == parent_path {
                            println!("!!!! e_path is parent path so skip");
                            None
                        } else {
                            println!(
                                "!!!! comp for {}, parent is {} and wanted parent is {}",
                                e_path.display(),
                                e_parent.display(),
                                parent_path.display()
                            );
                            if e_parent == parent_path {
                                println!(">yes");
                                Some(*ino)
                            } else {
                                println!(">no");
                                None
                            }
                        }
                    })
                    .collect();
                println!("LISTING RESULT {:?}", ino_list);
                Ok(ino_list)
            }
            None => {
                println!("ERROR IN RESULT");
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Parent inode not found",
                ))
            }
        }
    }

    // used directly in FuseControler's readdir function
    pub fn fs_readdir(&self, parent_ino: InodeIndex) -> io::Result<Vec<(InodeIndex, &FsEntry)>> {
        println!("fs_readdir called on ino {}", parent_ino); // DEBUG
        match self.list_files(parent_ino) {
            Ok(list) => Ok(list
                .into_iter()
                .filter_map(|inode| match self.index.get(&inode) {
                    Some(entry) => Some((inode, entry)),
                    None => None,
                })
                .collect()),
            Err(e) => Err(e),
        }
    }

    // use real fs metadata and traduct part of it to the fuse FileAttr metadata
    fn modify_metadata_template(data: openat::Metadata, ino: InodeIndex) -> FileAttr {
        let mut attr = TEMPLATE_FILE_ATTR;
        attr.ino = ino;
        attr.kind = if data.is_dir() {
            fuser::FileType::Directory
        } else if data.is_file() {
            fuser::FileType::RegularFile
        } else {
            fuser::FileType::CharDevice // random to detect unsupported
        };
        attr.size = data.len();
        attr
    }

    // get the metadata of a file from it's inode
    pub fn get_metadata(&self, ino: InodeIndex) -> io::Result<FileAttr> {
        println!("get_metadata called on ino {}", ino); // DEBUG
        match self.mirror_path_from_inode(ino) {
            Ok(path) => {
                println!("....GET METADATA FOR PATH MIRROR {:?}", path);
                match self.metal_handle.metadata(&path) {
                    Ok(data) => Ok(Self::modify_metadata_template(data, ino)),
                    Err(e) => Err(e),
                }
            }
            Err(e) => {
                println!("....mirror path from inode FAILED");
                Err(e)
            }
        }
    }

    pub fn fs_lookup(&self, parent_ino: InodeIndex, file_name: &OsStr) -> io::Result<FileAttr> {
        println!(
            "fs_lookup called on ino {} and file name {:#?}",
            parent_ino, file_name
        ); // DEBUG
        let file_name = file_name.to_string_lossy().to_string();
        match self.fs_readdir(parent_ino) {
            Ok(datas) => {
                let mut metadata: io::Result<FileAttr> =
                    Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
                for (ino, entry) in datas {
                    println!("looping on {:?}", entry);
                    if entry.get_name()?.to_string_lossy() == file_name {
                        metadata = self.get_metadata(ino);
                    };
                }
                metadata
            }
            Err(e) => Err(e),
        }
    }
}
