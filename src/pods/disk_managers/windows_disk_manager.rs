use std::{ffi::OsString, mem::MaybeUninit, os::windows::prelude::FileExt, path::Path};

use tokio::io;

use windows::{
    core::HSTRING,
    Wdk::Storage::FileSystem::{FileFsSizeInformation, NtQueryVolumeInformationFile},
    Win32::{
        Foundation::{GetLastError, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
    },
};
use winfsp::util::Win32SafeHandle;

use crate::pods::whpath::WhPath;
use windows::Wdk::System::SystemServices::FILE_FS_SIZE_INFORMATION;
use windows::Win32::System::IO::IO_STATUS_BLOCK;

use super::{DiskManager, DiskSizeInfo};

#[derive(Debug)]
pub struct WindowsDiskManager {
    handle: Win32SafeHandle,
    mount_point: WhPath, // mountpoint on linux and mirror mountpoint on windows
}

impl WindowsDiskManager {
    pub fn new(mount_point: WhPath) -> io::Result<Self> {
        let (parent, name) = mount_point.split_folder_file();
        let mut mount_point = WhPath::from(&parent);
        mount_point.push(&format!(".{name}"));

        let path = HSTRING::from(OsString::from(&mount_point.inner));

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
    fn new_file(&self, path: &WhPath, permissions: u16) -> io::Result<()> {
        std::fs::File::create(&self.mount_point.join(path).inner)?;
        Ok(())
    }

    fn remove_file(&self, path: &WhPath) -> io::Result<()> {
        std::fs::remove_file(&self.mount_point.join(path).inner)
    }

    fn remove_dir(&self, path: &WhPath) -> io::Result<()> {
        std::fs::remove_dir(&self.mount_point.join(path).inner)
    }

    fn write_file(&self, path: &WhPath, binary: &[u8], offset: usize) -> io::Result<usize> {
        return std::fs::File::open(&self.mount_point.join(path).inner)?
            .seek_write(binary, offset as u64);
    }

    fn set_file_size(&self, path: &WhPath, size: usize) -> io::Result<()> {
        std::fs::File::open(&self.mount_point.join(path).inner)?.set_len(size as u64)
    }

    fn mv_file(&self, path: &WhPath, new_path: &WhPath) -> io::Result<()> {
        // let mut original_path = path.clone(); // NOTE - Would be better if rename was non mutable
        // original_path.rename(new_name);
        std::fs::rename(
            &self.mount_point.join(path).inner,
            &self.mount_point.join(new_path).inner,
        )
    }

    fn read_file(&self, path: &WhPath, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        std::fs::File::open(&self.mount_point.join(path).inner)?.seek_read(buf, offset as u64)
    }

    fn new_dir(&self, path: &WhPath, permissions: u16) -> io::Result<()> {
        std::fs::create_dir(&self.mount_point.join(path).inner)
    }

    fn size_info(&self) -> std::io::Result<super::DiskSizeInfo> {
        self.get_volume_info_inner()
    }

    fn log_arbo(&self, path: &WhPath) -> std::io::Result<()> {
        todo!()
    }

    fn set_permisions(&self, path: &WhPath, permissions: u16) -> io::Result<()> {
        Ok(())
    }
}
