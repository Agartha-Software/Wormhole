use fuser::{FileAttr, FileType};
use libc::{S_IREAD, S_IWRITE};
use std::{
    ffi::OsStr,
    io::{self, Write},
    os::unix::fs::FileExt,
    path::PathBuf,
};

use crate::{
    network::message::{self, File, Folder, MessageContent, ToNetworkMessage},
    providers,
};

use super::{FsEntry, InodeIndex, Provider, TEMPLATE_FILE_ATTR};

impl Provider {
    pub fn mkfile(&mut self, parent_ino: InodeIndex, name: &OsStr) -> io::Result<FileAttr> {
        println!("MKFILE FUNCTION");
        self.check_file_type(parent_ino, FileType::Directory)?;
        println!("MKFILE FUNCTION1");

        let new_path = self.mirror_path_from_inode(parent_ino)?.join(name);
        println!("MKFILE FUNCTION2");
        println!("Creating file at {}", new_path.display()); // DEBUG
        match self.metal_handle.new_file(&new_path, 0o644) {
            Ok(_) => (),
            Err(e) => {
                println!("ERROR is {}", e);
                return Err(e);
            }
        }; // REVIEW look more in c mode_t value
        println!("MKFILE FUNCTION3");

        // add entry to the index
        self.index
            .insert(self.next_inode, FsEntry::File(new_path.clone(), vec![]));
        self.tx
            .send(ToNetworkMessage::BroadcastMessage(MessageContent::File(
                message::File {
                    path: new_path.into(),
                    ino: self.next_inode,
                },
            )))
            .expect("mkfile: unable to update modification on the network");

        // creating metadata to return
        let mut new_attr = TEMPLATE_FILE_ATTR;
        new_attr.ino = self.next_inode;
        new_attr.kind = FileType::RegularFile;
        new_attr.size = 0;
        self.next_inode += 1; // NOTE - ne jamais oublier d'incrémenter si besoin next_inode
        Ok(new_attr)
    }

    pub fn mkdir(&mut self, parent_ino: InodeIndex, name: &OsStr) -> io::Result<FileAttr> {
        self.check_file_type(parent_ino, FileType::Directory)?;
        // generation of the real path (of the mirror)
        let new_path = PathBuf::from(self.mirror_path_from_inode(parent_ino).unwrap()).join(name);

        // bare metal dir creation (on the mirror)
        self.metal_handle.create_dir(&new_path, 0o755)?; // REVIEW look more in c mode_t value
        println!("creating dir at {}", new_path.display()); // DEBUG

        // adding path to the wormhole index
        self.index
            .insert(self.next_inode, FsEntry::Directory(new_path.clone()));

        // send update to network
        self.tx
            .send(ToNetworkMessage::BroadcastMessage(
                MessageContent::NewFolder(Folder {
                    ino: self.next_inode,
                    path: new_path,
                }),
            ))
            .expect("mkdir: unable to update modification on the network");

        // creating metadata to return
        let mut new_attr = TEMPLATE_FILE_ATTR;
        new_attr.ino = self.next_inode;
        new_attr.kind = FileType::Directory;
        new_attr.size = 0;
        self.next_inode += 1; // NOTE - ne jamais oublier d'incrémenter si besoin next_inode

        Ok(new_attr)
    }

    pub fn rmfile(&mut self, parent_ino: InodeIndex, name: &OsStr) -> io::Result<()> {
        let (ino, _) = self.filesystem_from_parent_ino_and_name(parent_ino, name)?;

        self.mirror_path_from_inode(ino)
            .and_then(|file_path| self.metal_handle.remove_file(&file_path))
            .map(|_| {
                self.tx
                    .send(ToNetworkMessage::BroadcastMessage(MessageContent::Remove(
                        ino,
                    )))
                    .unwrap();
                self.index.remove(&ino);
            })
    }

    pub fn rmdir(&mut self, parent_ino: u64, name: &OsStr) -> io::Result<()> {
        let (ino, _) = self.filesystem_from_parent_ino_and_name(parent_ino, name)?;
        match self.fs_readdir(ino) {
            Ok(files_system) => {
                if files_system.len() > 0 {
                    return Err(io::Error::new(io::ErrorKind::Other, "Folder not empty"));
                }
                self.mirror_path_from_inode(ino)
                    .and_then(|file_path| self.metal_handle.remove_dir(&file_path))
                    .map(|_| {
                        self.tx
                            .send(ToNetworkMessage::BroadcastMessage(MessageContent::Remove(
                                ino,
                            )))
                            .unwrap();
                        self.index.remove(&ino);
                    })
            }
            Err(e) => {
                println!("ERROR DURING THE FS_READDIR IN RMDIR");
                Err(e)
            }
        }
    }

    pub fn rename(
        &mut self,
        parent_ino: InodeIndex,
        name: &OsStr,
        newparent_ino: InodeIndex,
        newname: &OsStr,
    ) -> Option<()> {
        let _ = newname;
        let _ = newparent_ino;
        let _ = name;
        let _ = parent_ino;
        // pas clair de quand c'est appelé, si ça l'est sur des dossiers
        // non vides, go ignorer pour l'instant
        Some(())
    }

    // returns the writed size
    pub fn write(&self, ino: InodeIndex, offset: i64, data: &[u8]) -> io::Result<u32> {
        match self.index.get(&ino) {
            Some(FsEntry::File(path, _)) => {
                //let path = self.mirror_path_from_inode(ino)?; // Absolute path
                let wrfile = self.metal_handle.write_file(path, S_IWRITE | S_IREAD)?;
                wrfile
                    .write_all_at(data, offset.try_into().unwrap())
                    .expect("can't write file");
                // fs::write(path, data)?;
                self.tx
                    .send(ToNetworkMessage::BroadcastMessage(MessageContent::Write(
                        ino,
                        data.to_owned(),
                    )))
                    .unwrap();
                Ok(data.len() as u32)
            }
            Some(_) => Err(io::Error::new(io::ErrorKind::Other, "File not writable")),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "File not found")),
        }
    }

    // RECEPTION
    // REVIEW - not yet refactored nor properly error handled
    pub fn new_folder(&mut self, ino: InodeIndex, path: PathBuf) {
        let real_path = PathBuf::from(self.local_source.clone()).join(&path);
        println!("Provider make new folder at: {:?}", real_path);
        self.metal_handle
            .create_dir(&real_path, S_IWRITE | S_IREAD)
            .expect("unable to create folder");
        // fs::create_dir(&real_path).unwrap();
        self.index.insert(ino, FsEntry::Directory(path));
    }

    pub fn new_file(&mut self, ino: InodeIndex, path: PathBuf) {
        println!("Provider make new file at ORIGINAL PATH: {:?}", path);
        // let real_path = PathBuf::from(self.local_source.clone()).join(&path);
        // println!("Provider make new file at: {:?}", real_path);
        self.metal_handle
            .new_file(&path, S_IWRITE | S_IREAD)
            .expect("unable to create file");
        self.index.insert(ino, FsEntry::File(path, vec![]));
        self.next_inode = ino + 1;
        println!("created created created");
    }

    pub fn recpt_remove(&mut self, ino: InodeIndex) {
        // let real_path = PathBuf::from(self.local_source.clone()).join(&path);
        match self.index.get(&ino).unwrap() {
            FsEntry::Directory(path) => self.metal_handle.remove_dir(path).unwrap(),
            FsEntry::File(path, _) => self.metal_handle.remove_file(path).unwrap(),
        }
        self.index.remove(&ino);
    }

    pub fn recpt_write(&mut self, ino: InodeIndex, content: Vec<u8>) {
        if let FsEntry::File(path, _) = self.index.get(&ino).unwrap() {
            println!("Provider write to file at: {:?}", path);
            let mut file = self
                .metal_handle
                .write_file(path, S_IWRITE | S_IREAD)
                .expect("can't write file");
            file.write_all(&content).unwrap();
        } else {
            panic!("Tried to write on not a file");
        }
    }

    // pub fn recpt_rename(&mut self, ino: InodeIndex, newparent_ino: InodeIndex, newname: &OsStr) {
    //     let (_, path) = self.index.get(&ino).unwrap();
    //     let real_path = PathBuf::from(self.local_source.clone()).join(&path);
    //     let real_path = PathBuf::from(self.local_source.clone()).join(&path);
    //     println!("Provider rename object at: {:?}", real_path);

    // }
}
