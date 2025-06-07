use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tokio::io;

use crate::pods::filesystem::fs_interface::SimpleFileType;
use crate::pods::whpath::WhPath;

use super::{DiskManager, DiskSizeInfo};

#[derive(PartialEq, Debug)]
pub enum VirtualFile {
    File(Vec<u8>),
    Folder(Vec<WhPath>),
}

impl Into<SimpleFileType> for &VirtualFile {
    fn into(self) -> SimpleFileType {
        match self {
            VirtualFile::File(_) => SimpleFileType::File,
            VirtualFile::Folder(_) => SimpleFileType::Directory,
        }
    }
}

#[derive(Debug)]
pub struct DummyDiskManager {
    files: Arc<RwLock<HashMap<WhPath, VirtualFile>>>,
    size: Arc<RwLock<usize>>,
    _mount_point: WhPath, // mountpoint on linux and mirror mountpoint on windows
}

impl DummyDiskManager {
    pub fn new(mount_point: &WhPath) -> io::Result<Self> {
        let mut folders = HashMap::new();
        folders.insert(WhPath::from("."), VirtualFile::Folder(vec![]));
        folders.insert(WhPath::from(""), VirtualFile::Folder(vec![]));
        Ok(Self {
            files: Arc::new(RwLock::new(folders)),
            _mount_point: mount_point.clone(),
            size: Arc::new(RwLock::new(0)),
        })
    }

    fn mv_recurse(&self, old_path: &WhPath, new_path: &WhPath) {
        let removed = self
            .files
            .write()
            .expect("VirtDisk::tree rwLock")
            .remove(old_path); // if let does not drop temporaries
        if let Some(file) = removed {
            if let VirtualFile::Folder(entries) = &file {
                entries.iter().for_each(|name| {
                    let mut old_path = old_path.clone();
                    old_path.push(&name.get_end());
                    let mut new_path = new_path.clone();
                    new_path.push(&name.get_end());
                    self.mv_recurse(&old_path, &new_path);
                });
            }
            log::trace!(
                "{} => {}, ({:?})",
                old_path,
                new_path,
                Into::<SimpleFileType>::into(&file)
            );
            self.files
                .try_write()
                .expect("VirtDisk::tree rwLock")
                .insert(new_path.clone(), file);
        } else {
            log::error!("VirtDisk::mv_recurse: \"{old_path}\" not found")
        }
    }
}

/// always takes a WhPath and infers the real disk path
impl DiskManager for DummyDiskManager {
    fn new_file(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        let path = path.clone().set_relative();
        let (f_path, _) = path.split_folder_file();
        let f_path: WhPath = (&f_path).into();
        let mut lock = self.files.write().expect("VirtDisk::new_file rwLock");
        match lock.get_mut(&f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.push(path.clone())),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        lock.insert(path, VirtualFile::File(Vec::new()));
        Ok(())
    }

    fn remove_file(&self, path: &WhPath) -> io::Result<()> {
        let path = path.clone().set_relative();
        let f_path = path.get_folder();
        let f_path: WhPath = (&f_path).into();

        let mut total_size = self.size.write().expect("VirtDisk::remove_file rwLock");

        let mut lock = self.files.write().expect("VirtDisk::remove_file rwLock");

        match lock.get_mut(&f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.retain(|v| v != &path)),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        if let Some(shrunk) = total_size.checked_sub(match lock.remove(&path) {
            Some(VirtualFile::File(vec)) => Ok::<usize, io::Error>(vec.len()),
            Some(VirtualFile::Folder(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?) {
            *total_size = shrunk;
        }
        Ok(())
    }

    fn mv_file(&self, old_path: &WhPath, new_path: &WhPath) -> io::Result<()> {
        let old_path = old_path.clone().set_relative();
        let f_old_path = old_path.get_folder();
        let f_old_path: WhPath = (&f_old_path).into();

        let new_path = new_path.clone().set_relative();
        let f_new_path = new_path.get_folder();
        let f_new_path: WhPath = (&f_new_path).into();

        {
            let mut lock = self.files.write().expect("VirtDisk::remove_file rwLock");

            match lock.get_mut(&f_old_path) {
                Some(VirtualFile::Folder(vec)) => {
                    Ok::<(), io::Error>(vec.retain(|v| v != &old_path))
                }
                Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
                None => Err(io::ErrorKind::NotFound.into()),
            }?;

            match lock.get_mut(&f_new_path) {
                Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.push(new_path.clone())),
                Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
                None => Err(io::ErrorKind::NotFound.into()),
            }?;
        }

        self.mv_recurse(&old_path, &new_path);
        Ok(())
    }

    fn remove_dir(&self, path: &WhPath) -> io::Result<()> {
        let path = path.clone().set_relative();
        let f_path = path.get_folder();
        let f_path: WhPath = (&f_path).into();

        let mut lock = self.files.write().expect("VirtDisk::remove_dir rwLock");

        match lock.get_mut(&f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.retain(|v| v != &path)),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        lock.remove(&path);
        Ok(())
    }

    fn write_file(&self, path: &WhPath, binary: &[u8], offset: usize) -> io::Result<usize> {
        if let Some(VirtualFile::File(file)) = self
            .files
            .write()
            .expect("VirtDisk::write_file rwLock")
            .get_mut(&path.clone().set_relative())
        {
            let len = binary.len();
            let grow = offset
                .checked_add(len)
                .and_then(|x| x.checked_sub(file.len()))
                .unwrap_or(0);
            *self.size.write().expect("VirtDisk::write_file rwLock") += grow;
            file.splice(
                (offset)..(std::cmp::min(file.len(), offset + len)),
                binary.iter().cloned(),
            );
            Ok(len)
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    }

    fn set_file_size(&self, path: &WhPath, size: usize) -> io::Result<()> {
        if let Some(VirtualFile::File(file)) = self
            .files
            .write()
            .expect("VirtDisk::write_file rwLock")
            .get_mut(&path.clone().set_relative())
        {
            let grow = (size > file.len()).then(|| size - file.len()).unwrap_or(0);
            let shrink = (file.len() > size).then(|| file.len() - size).unwrap_or(0);
            {
                let mut total_size = self.size.write().expect("VirtDisk::write_file rwLock");
                *total_size += grow;
                *total_size -= shrink; // TODO: this can panic and poison on underflow from desynced size
            }

            file.resize(size, 0);
            Ok(())
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    }

    fn read_file(&self, path: &WhPath, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(VirtualFile::File(file)) = self
            .files
            .read()
            .expect("VirtDisk::read_file rwLock")
            .get(&path.clone().set_relative())
        {
            let len = std::cmp::min(buf.len(), file.len() - offset);
            buf[0..len].copy_from_slice(&file[(offset)..(offset + len)]);
            Ok(len)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "file storage not found",
            ))
        }
    }

    fn new_dir(&self, path: &WhPath, _permissions: u16) -> io::Result<()> {
        let path = path.clone().set_relative();
        let (f_path, _) = path.split_folder_file();
        let f_path: WhPath = (&f_path).into();
        let mut lock = self.files.write().expect("VirtDisk::new_file rwLock");
        match lock.get_mut(&f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.push(path.clone())),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        lock.insert(path.clone().set_relative(), VirtualFile::Folder(vec![]));
        Ok(())
    }

    fn size_info(&self) -> io::Result<DiskSizeInfo> {
        let s = sysinfo::System::new_all();
        Ok(DiskSizeInfo {
            free_size: s.available_memory() as usize,
            total_size: self
                .size
                .read()
                .map(|s| *s)
                .map_err(|_| io::Error::new(io::ErrorKind::Other.into(), "poison error"))?,
        })
    }

    fn log_arbo(&self, path: &WhPath) -> io::Result<()> {
        let path: WhPath = path.clone().set_relative();

        let lock = self.files.read().expect("VirtDisk::log_arbo rwLock");

        match lock.get(&path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>({
                vec.iter().for_each(|f| {
                    let t = match lock.get(f) {
                        Some(VirtualFile::File(_)) => format!("{:?}", SimpleFileType::File),
                        Some(VirtualFile::Folder(_)) => format!("{:?}", SimpleFileType::Directory),
                        None => "err".into(),
                    };
                    log::debug!("|{:?} => {}|", f.get_end(), t);
                });
            }),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }
    }

    fn set_permisions(&self, _path: &WhPath, _permissions: u16) -> io::Result<()> {
        Ok(())
    }
}
