use super::{FileAttr, FileType};
use libc::{O_WRONLY, S_IREAD, S_IWRITE};
use std::{
    ffi::OsStr, io::{self, Write}, os::windows::fs::FileExt, path::PathBuf
};

use crate::network::message::{self, Folder, NetworkMessage};

use super::{Provider, TEMPLATE_FILE_ATTR};

impl Provider {
    pub fn mkfile(&mut self, parent_ino: u64, name: &OsStr) -> io::Result<FileAttr> {
        println!("MKFILE FUNCTION");
        self.check_file_type(parent_ino, FileType::Directory)?;
        println!("MKFILE FUNCTION1");

        let new_path = self.mirror_path_from_inode(parent_ino)?.join(name);
        println!("MKFILE FUNCTION2");
        println!("Creating file at {}", new_path.display()); // DEBUG
        match self.folder_handle.new_file(&new_path, 0o644) {
            Ok(_) => (),
            Err(e) => {
                println!("ERROR is {}", e);
                return Err(e);
            }
        }; // REVIEW look more in c mode_t value
        println!("MKFILE FUNCTION3");

        // add entry to the index
        self.index
            .insert(self.next_inode, (FileType::RegularFile, new_path.clone()));
        self.tx
            .send(NetworkMessage::File(message::File {
                path: new_path.into(),
                file: [].to_vec(), // REVIEW - why this field ? useful ?
                ino: self.next_inode,
            }))
            .expect("mkfile: unable to update modification on the network");

        // creating metadata to return
        let mut new_attr = TEMPLATE_FILE_ATTR;
        new_attr.ino = self.next_inode;
        new_attr.kind = FileType::RegularFile;
        new_attr.size = 0;
        self.next_inode += 1; // NOTE - ne jamais oublier d'incrémenter si besoin next_inode
        Ok(new_attr)
    }

    pub fn mkdir(&mut self, parent_ino: u64, name: &OsStr) -> io::Result<FileAttr> {
        self.check_file_type(parent_ino, FileType::Directory)?;
        // generation of the real path (of the mirror)
        let new_path = PathBuf::from(self.mirror_path_from_inode(parent_ino).unwrap()).join(name);

        // bare metal dir creation (on the mirror)
        self.folder_handle.create_dir(&new_path, 0o755)?; // REVIEW look more in c mode_t value
        println!("creating dir at {}", new_path.display()); // DEBUG

        // adding path to the wormhole index
        self.index
            .insert(self.next_inode, (FileType::Directory, new_path.clone()));

        // send update to network
        self.tx
            .send(NetworkMessage::NewFolder(Folder {
                ino: self.next_inode,
                path: new_path,
            }))
            .expect("mkdir: unable to update modification on the network");

        // creating metadata to return
        let mut new_attr = TEMPLATE_FILE_ATTR;
        new_attr.ino = self.next_inode;
        new_attr.kind = FileType::Directory;
        new_attr.size = 0;
        self.next_inode += 1; // NOTE - ne jamais oublier d'incrémenter si besoin next_inode

        Ok(new_attr)
    }

    pub fn rmfile(&mut self, parent_ino: u64, name: &OsStr) -> io::Result<()> {
        let file = self.file_from_parent_ino_and_name(parent_ino, name)?;

        self.mirror_path_from_inode(file.0)
            .and_then(|file_path| self.folder_handle.remove_file(&file_path))
            .map(|_| {
                self.tx.send(NetworkMessage::Remove(file.0)).unwrap();
                self.index.remove(&file.0);
            })
    }

    pub fn rmdir(&mut self, parent_ino: u64, name: &OsStr) -> Option<()> {
        let _ = name;
        let _ = parent_ino;
        // should only be called on empty folders
        // if 404, not empty or file -> None
        Some(())
    }

    pub fn rename(
        &mut self,
        parent_ino: u64,
        name: &OsStr,
        newparent_ino: u64,
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
    pub fn write(&self, ino: u64, offset: i64, data: &[u8]) -> io::Result<u32> {
        match self.index.get(&ino) {
            Some((FileType::RegularFile, _)) => {
                let path = self.mirror_path_from_inode(ino)?;
                let wrfile = self.folder_handle.write_file(&path, S_IWRITE | S_IREAD)?;
                #[cfg(target_os = "windows")]
                wrfile
                    .seek_write(data, offset.try_into().unwrap())
                    .expect("can't write file");
                #[cfg(target_os = "linux")]
                wrfile
                    .write_all_at(data, offset.try_into().unwrap())
                    .expect("can't write file");
                // fs::write(path, data)?;
                self.tx
                    .send(NetworkMessage::Write(ino, data.to_owned()))
                    .unwrap();
                Ok(data.len() as u32)
            }
            Some(_) => Err(io::Error::new(io::ErrorKind::Other, "File not writable")),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "File not found")),
        }
    }

    // RECEPTION
    // REVIEW - not yet refactored nor properly error handled
    pub fn new_folder(&mut self, ino: u64, path: PathBuf) {
        let real_path = PathBuf::from(self.local_source.clone()).join(&path);
        println!("Provider make new folder at: {:?}", real_path);
        self.folder_handle
            .create_dir(&real_path, S_IWRITE | S_IREAD)
            .expect("unable to create folder");
        // fs::create_dir(&real_path).unwrap();
        self.index.insert(ino, (FileType::Directory, path));
    }

    pub fn new_file(&mut self, ino: u64, path: PathBuf) {
        println!("Provider make new file at ORIGINAL PATH: {:?}", path);
        // let real_path = PathBuf::from(self.local_source.clone()).join(&path);
        // println!("Provider make new file at: {:?}", real_path);
        self.folder_handle
            .new_file(&path, S_IWRITE | S_IREAD)
            .expect("unable to create file");
        self.index.insert(ino, (FileType::RegularFile, path));
        self.next_inode = ino + 1;
        println!("created created created");
    }

    pub fn recpt_remove(&mut self, ino: u64) {
        let (file_type, path) = self.index.get(&ino).unwrap();
        // let real_path = PathBuf::from(self.local_source.clone()).join(&path);
        println!("Provider remove object at: {:?}", path);
        match file_type {
            FileType::Directory => todo!(),
            FileType::RegularFile => self.folder_handle.remove_file(path).unwrap(),
            _ => todo!(),
        }
        self.index.remove(&ino);
    }

    pub fn recpt_write(&mut self, ino: u64, content: Vec<u8>) {
        let (_, path) = self.index.get(&ino).unwrap();
        // let real_path = PathBuf::from(self.local_source.clone()).join(&path);
        println!("Provider write to file at: {:?}", path);
        let mut file = self
            .folder_handle
            .write_file(path, S_IWRITE | S_IREAD)
            .expect("can't write file");
        file.write_all(&content).unwrap();
    }

    // pub fn recpt_rename(&mut self, ino: u64, newparent_ino: u64, newname: &OsStr) {
    //     let (_, path) = self.index.get(&ino).unwrap();
    //     let real_path = PathBuf::from(self.local_source.clone()).join(&path);
    //     let real_path = PathBuf::from(self.local_source.clone()).join(&path);
    //     println!("Provider rename object at: {:?}", real_path);

    // }
}
