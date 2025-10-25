use std::{ffi::OsString, mem::MaybeUninit, os::windows::prelude::FileExt, path::{Path, PathBuf}};

use tokio::io;

use windows::{
    core::HSTRING,
    Wdk::Storage::FileSystem::{FileFsSizeInformation, NtQueryVolumeInformationFile},
    Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
    },
};
use winfsp::util::Win32SafeHandle;

use windows::Wdk::System::SystemServices::FILE_FS_SIZE_INFORMATION;
use windows::Win32::System::IO::IO_STATUS_BLOCK;

use crate::winfsp::winfsp_impl::aliased_path;

use super::{DiskManager, DiskSizeInfo};

#[derive(Debug)]
pub struct WindowsDiskManager {
    handle: Win32SafeHandle,
    mount_point: PathBuf, // mountpoint on linux and mirror mountpoint on windows
}

impl WindowsDiskManager {
    pub fn new(mount_point: &Path) -> io::Result<Self> {
        // FIXME - monting on path/.dir instead of path/dir ?
        let mount_point = aliased_path(mount_point).map_err(|_| io::ErrorKind::InvalidFilename)?;

        let path = HSTRING::from(OsString::from(&mount_point));

        let handle = unsafe {
            CreateFileW(
                &path,
                FILE_READ_ATTRIBUTES.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )?
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        Ok(Self {
            mount_point,
            handle: Win32SafeHandle::from(handle),
        })
    }

    fn get_volume_info_inner(&self) -> io::Result<DiskSizeInfo> {
        let mut iosb: MaybeUninit<IO_STATUS_BLOCK> = MaybeUninit::zeroed();
        let mut fsize_info: MaybeUninit<FILE_FS_SIZE_INFORMATION> = MaybeUninit::zeroed();

        let fsize_info = unsafe {
            NtQueryVolumeInformationFile(
                *self.handle,
                iosb.as_mut_ptr(),
                fsize_info.as_mut_ptr().cast(),
                size_of::<FILE_FS_SIZE_INFORMATION>() as u32,
                FileFsSizeInformation,
            )
            .ok()?;

            fsize_info.assume_init()
        };

        let sector_size = fsize_info.BytesPerSector;
        let sectors_per_alloc_unit = fsize_info.SectorsPerAllocationUnit;
        let alloc_unit = sector_size * sectors_per_alloc_unit;

        Ok(DiskSizeInfo {
            free_size: fsize_info.TotalAllocationUnits as usize * alloc_unit as usize,
            total_size: fsize_info.AvailableAllocationUnits as usize * alloc_unit as usize,
        })
    }
}

/// always takes a WhPath and infers the real disk path
impl DiskManager for WindowsDiskManager {
    fn new_file(&self, path: &Path, permissions: u16) -> io::Result<()> {
        std::fs::File::create(&self.mount_point.join(path))?;
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(&self.mount_point.join(path))
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir(&self.mount_point.join(path))
    }

    fn write_file(&self, path: &Path, binary: &[u8], offset: usize) -> io::Result<usize> {
        return std::fs::File::open(&self.mount_point.join(path))?
            .seek_write(binary, offset as u64);
    }

    fn set_file_size(&self, path: &Path, size: usize) -> io::Result<()> {
        std::fs::File::open(&self.mount_point.join(path))?.set_len(size as u64)
    }

    fn mv_file(&self, path: &Path, new_path: &Path) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        std::fs::rename(
            &self.mount_point.join(path),
            &self.mount_point.join(new_path),
        )
    }

    fn read_file(&self, path: &Path, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        std::fs::File::open(&self.mount_point.join(path))?.seek_read(buf, offset as u64)
    }

    fn new_dir(&self, path: &Path, permissions: u16) -> io::Result<()> {
        std::fs::create_dir(&self.mount_point.join(path))
    }

    fn size_info(&self) -> std::io::Result<super::DiskSizeInfo> {
        self.get_volume_info_inner()
    }

    fn log_arbo(&self, path: &Path) -> std::io::Result<()> {
        todo!()
    }

    fn set_permisions(&self, path: &Path, permissions: u16) -> io::Result<()> {
        Ok(())
    }
}
