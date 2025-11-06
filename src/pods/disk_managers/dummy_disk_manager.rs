use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use tokio::io;

use crate::pods::filesystem::fs_interface::SimpleFileType;

use super::{DiskManager, DiskSizeInfo};

#[derive(PartialEq, Debug)]
pub enum VirtualFile {
    File(Vec<u8>),
    Folder(Vec<PathBuf>),
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
    files: Arc<RwLock<HashMap<PathBuf, VirtualFile>>>,
    size: Arc<RwLock<usize>>,
    _mount_point: PathBuf, // mountpoint on linux and mirror mountpoint on windows
}

// NOTE -> before refactor was using WhPath::set_relative each time
impl DummyDiskManager {
    pub fn new(mount_point: &Path) -> io::Result<Self> {
        let mut folders = HashMap::new();
        folders.insert(".".into(), VirtualFile::Folder(vec![]));
        folders.insert(PathBuf::new(), VirtualFile::Folder(vec![]));
        Ok(Self {
            files: Arc::new(RwLock::new(folders)),
            _mount_point: mount_point.to_owned(),
            size: Arc::new(RwLock::new(0)),
        })
    }

    fn mv_recurse(&self, old_path: &Path, new_path: &Path) {
        let removed = self
            .files
            .write()
            .expect("VirtDisk::tree rwLock")
            .remove(old_path); // if let does not drop temporaries
        if let Some(file) = removed {
            if let VirtualFile::Folder(entries) = &file {
                entries.iter().for_each(|name| {
                    let mut old_path = old_path.to_owned();
                    old_path.push(name.file_name().expect("no file name"));
                    let mut new_path = new_path.to_owned();
                    new_path.push(name.file_name().expect("no file name"));
                    self.mv_recurse(&old_path, &new_path);
                });
            }
            log::trace!(
                "{:?} => {:?}, ({:?})",
                old_path,
                new_path,
                Into::<SimpleFileType>::into(&file)
            );
            self.files
                .try_write()
                .expect("VirtDisk::tree rwLock")
                .insert(new_path.to_owned(), file);
        } else {
            log::error!("VirtDisk::mv_recurse: \"{old_path:?}\" not found")
        }
    }
}

impl DiskManager for DummyDiskManager {
    fn new_file(&self, path: &Path, _permissions: u16) -> io::Result<()> {
        let f_path = path.parent().expect("no parent");
        let mut lock = self.files.write().expect("VirtDisk::new_file rwLock");

        match lock.get_mut(f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.push(path.to_owned())),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        let old = lock.insert(path.to_owned(), VirtualFile::File(Vec::new()));
        match old {
            None => (),
            Some(VirtualFile::File(data)) => {
                let mut size = self.size.write().expect("new_file");
                *size = size.checked_sub(data.len()).unwrap_or(0);
            }
            Some(VirtualFile::Folder(_)) => return Err(io::ErrorKind::AlreadyExists.into()),
        }
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let f_path = path.parent().expect("no parent");

        let mut total_size = self.size.write().expect("VirtDisk::remove_file rwLock");

        let mut lock = self.files.write().expect("VirtDisk::remove_file rwLock");

        match lock.get_mut(f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.retain(|v| v != path)),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        if let Some(shrunk) = total_size.checked_sub(match lock.remove(path) {
            Some(VirtualFile::File(vec)) => Ok::<usize, io::Error>(vec.len()),
            Some(VirtualFile::Folder(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?) {
            *total_size = shrunk;
        }
        Ok(())
    }

    fn mv_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        let f_old_path = old_path.parent().expect("no parent");
        let f_new_path = new_path.parent().expect("no parent");

        {
            let mut lock = self.files.write().expect("VirtDisk::remove_file rwLock");

            match lock.get_mut(f_old_path) {
                Some(VirtualFile::Folder(vec)) => {
                    Ok::<(), io::Error>(vec.retain(|v| v != old_path))
                }
                Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
                None => Err(io::ErrorKind::NotFound.into()),
            }?;

            match lock.get_mut(f_new_path) {
                Some(VirtualFile::Folder(vec)) => {
                    Ok::<(), io::Error>(vec.push(new_path.to_owned()))
                }
                Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
                None => Err(io::ErrorKind::NotFound.into()),
            }?;
        }

        self.mv_recurse(&old_path, &new_path);
        Ok(())
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        let f_path = path.parent().expect("no parent");

        let mut lock = self.files.write().expect("VirtDisk::remove_dir rwLock");

        match lock.get_mut(f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.retain(|v| v != path)),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        lock.remove(path);
        Ok(())
    }

    fn write_file(&self, path: &Path, binary: &[u8], offset: usize) -> io::Result<usize> {
        if let Some(VirtualFile::File(file)) = self
            .files
            .write()
            .expect("VirtDisk::write_file rwLock")
            .get_mut(path)
        {
            let len = binary.len();
            let grow = offset
                .checked_add(len)
                .and_then(|end| end.checked_sub(file.len()))
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

    fn set_file_size(&self, path: &Path, size: usize) -> io::Result<()> {
        if let Some(VirtualFile::File(file)) = self
            .files
            .write()
            .expect("VirtDisk::write_file rwLock")
            .get_mut(path)
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

    fn read_file(&self, path: &Path, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(VirtualFile::File(file)) = self
            .files
            .read()
            .expect("VirtDisk::read_file rwLock")
            .get(path)
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

    fn new_dir(&self, path: &Path, _permissions: u16) -> io::Result<()> {
        let f_path = path.parent().expect("no parent");
        let mut lock = self.files.write().expect("VirtDisk::new_file rwLock");

        match lock.get_mut(f_path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>(vec.push(path.to_owned())),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }?;
        lock.insert(path.to_owned(), VirtualFile::Folder(vec![]));
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

    fn log_arbo(&self, path: &Path) -> io::Result<()> {
        let lock = self.files.read().expect("VirtDisk::log_arbo rwLock");

        match lock.get(path) {
            Some(VirtualFile::Folder(vec)) => Ok::<(), io::Error>({
                vec.iter().for_each(|f| {
                    let t = match lock.get(f) {
                        Some(VirtualFile::File(_)) => format!("{:?}", SimpleFileType::File),
                        Some(VirtualFile::Folder(_)) => format!("{:?}", SimpleFileType::Directory),
                        None => "err".into(),
                    };
                    log::debug!("|{:?} => {}|", f.file_name(), t);
                });
            }),
            Some(VirtualFile::File(_)) => Err(io::ErrorKind::InvalidData.into()),
            None => Err(io::ErrorKind::NotFound.into()),
        }
    }

    fn set_permisions(&self, _path: &Path, _permissions: u16) -> io::Result<()> {
        Ok(())
    }

    fn file_exists(&self, path: &WhPath) -> bool {
        self.files
            .read()
            .expect("VirtDisk::read_file rwLock")
            .get(&path.clone().set_relative())
            .is_some()
    }
}
