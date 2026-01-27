use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use custom_error::custom_error;
use futures::io;
use nt_time::FileTime;
use ntapi::ntioapi::FILE_DIRECTORY_FILE;
use windows::Win32::Foundation::{
    STATUS_INVALID_DEVICE_REQUEST, STATUS_OBJECT_NAME_EXISTS, STATUS_OBJECT_NAME_NOT_FOUND,
};
use winfsp::{
    filesystem::{DirInfo, FileInfo, FileSecurity, FileSystemContext, WideNameInfo},
    host::{FileSystemHost, VolumeParams},
};
use winfsp_sys::{FspCleanupDelete, FILE_ACCESS_RIGHTS};

use crate::pods::{
    filesystem::file_handle::{AccessMode, FileHandleManager, OpenFlags},
    itree::FsEntry,
};
use crate::{
    error::WhError,
    pods::{
        filesystem::fs_interface::FsInterface,
        itree::{ITree, Ino, WINDOWS_DEFAULT_PERMS_MODE},
        whpath::{ConversionError, InodeName, WhPath, WhPathError},
    },
};

#[derive(PartialEq, Debug)]
pub struct WormholeHandle {
    pub ino: Ino,
    pub handle: u64,
}

pub struct FSPController {
    pub volume_label: Arc<RwLock<String>>,
    pub fs_interface: Arc<FsInterface>,
}

#[allow(unused)] // unused: field `0` is used through ffi
pub struct WinfspHost(FileSystemHost<FSPController>);

impl std::fmt::Debug for WinfspHost {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

custom_error! {pub AliasedPathError
    NoFolderName = "Can't get folder name",
    WhError{source: WhError} = "{source}",
}

/// Add a '.' the last element (dir or file name): /test/dir => /test/.dir
pub(crate) fn aliased_path(path: &Path) -> Result<PathBuf, AliasedPathError> {
    let mut buf = path.to_owned();
    let mut file_name = OsString::from(".");

    file_name.push(path.file_name().ok_or(AliasedPathError::NoFolderName)?);
    buf.set_file_name(file_name);

    Ok(buf)
}

impl Drop for FSPController {
    fn drop(&mut self) {
        log::trace!("Drop of FSPController");
    }
}

impl FSPController {
    fn get_file_info_internal(
        &self,
        context: &WormholeHandle,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        let itree = ITree::read_lock(
            &self.fs_interface.network_interface.itree,
            "winfsp::get_file_info",
        )?;

        let inode = itree.get_inode(context.ino)?;
        *file_info = (&inode.meta).into();
        Ok(())
    }
}

pub fn mount_fsp(fs_interface: Arc<FsInterface>) -> Result<WinfspHost, std::io::Error> {
    let volume_params = VolumeParams::default();
    let mountpoint = fs_interface.mountpoint.clone();

    let wormhole_context = FSPController {
        volume_label: Arc::new(RwLock::new("wormhole_fs".into())),
        fs_interface,
    };
    let mut host = FileSystemHost::<FSPController>::new(volume_params, wormhole_context)
        .map_err(|_| std::io::Error::other("WinFSP FileSystemHost::new error"))?;

    let path = mountpoint.to_string_lossy().to_string().replace("\\", "/");
    log::info!("WinFSP mounting host @ {:?} ...", &path);
    host.mount(&path)
        .map_err(|_| io::Error::other("WinFSP mount error"))?;

    host.start_with_threads(1)
        .map_err(|_| io::Error::other("WinFSP start_with_threads error"))?;
    Ok(WinfspHost(host))
}

impl FileSystemContext for FSPController {
    type FileContext = WormholeHandle;

    fn get_security_by_name(
        &self,
        file_name: &winfsp::U16CStr,
        _security_descriptor: Option<&mut [std::ffi::c_void]>,
        reparse_point_resolver: impl FnOnce(
            &winfsp::U16CStr,
        ) -> Option<winfsp::filesystem::FileSecurity>,
    ) -> winfsp::Result<winfsp::filesystem::FileSecurity> {
        // thread::sleep(std::time::Duration::from_secs(2));
        log::trace!(
            "winfsp::get_security_by_name({})",
            file_name.to_string_lossy(),
        );

        if let Some(security) = reparse_point_resolver(file_name) {
            log::trace!("ok({:?})", security);
            return Ok(security);
        }

        let path = WhPath::from_fake_absolute(file_name)?;

        let file_info: FileInfo = (&ITree::read_lock(
            &self.fs_interface.network_interface.itree,
            "get_security_by_name",
        )?
        .get_inode_from_path(&path)
        .inspect_err(|e| log::trace!("{:?}:{:?}", &path, e))?
        .meta)
            .into();
        // let mut descriptor_size = 0;
        // let option_sd = if security_descriptor.is_some() {
        //     Some(
        //         self.dummy_file
        //             .security_descriptor(SecurityInformation::all()).map_err(|e| {log::error!("{}:{:?}", &self.dummy_file.to_string_lossy(), e); e})?
        //     )
        // } else {
        //     None
        // };
        // if let (Some(sec_dec_storage), Some(got_sd)) = (security_descriptor, option_sd) {
        //     descriptor_size = unsafe {
        //         winapi::um::securitybaseapi::GetSecurityDescriptorLength(got_sd.as_ptr() as *mut _)
        //     } as usize;

        //     if sec_dec_storage.len() >= descriptor_size {
        //         unsafe {
        //             (got_sd.as_ptr() as *mut u8)
        //                 .copy_to(sec_dec_storage.as_ptr() as *mut u8, descriptor_size);
        //         }
        //     };
        // }
        let sec = FileSecurity {
            reparse: false,
            sz_security_descriptor: 0,
            attributes: file_info.file_attributes,
        };
        log::trace!("ok({:?})", sec);
        winfsp::Result::Ok(sec)
    }

    fn open(
        &self,
        file_name: &winfsp::U16CStr,
        _create_options: u32,
        granted_access: FILE_ACCESS_RIGHTS,
        file_info: &mut winfsp::filesystem::OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        // thread::sleep(std::time::Duration::from_secs(2));
        let display_name = file_name.display();
        log::trace!("open({display_name})");

        let path = WhPath::from_fake_absolute(file_name)?;
        let inode = ITree::read_lock(&self.fs_interface.network_interface.itree, "winfsp::open")?
            .get_inode_from_path(&path)
            .inspect_err(|e| log::warn!("open({display_name})::{e};"))?
            .clone();
        *file_info.as_mut() = (&inode.meta).into();
        file_info.set_normalized_name(file_name.as_slice(), None);
        let handle = self
            .fs_interface
            .open(
                inode.id,
                OpenFlags::from_win_u32(granted_access),
                AccessMode::from_win_u32(granted_access),
            )
            .inspect_err(|e| log::warn!("open({display_name})::{e}"))?;
        log::trace!("ok:{};", inode.id);
        Ok(WormholeHandle {
            ino: inode.id,
            handle,
        })
    }

    fn create(
        &self,
        file_name: &winfsp::U16CStr,
        create_options: u32,
        granted_access: FILE_ACCESS_RIGHTS,
        _file_attributes: winfsp_sys::FILE_FLAGS_AND_ATTRIBUTES,
        _security_descriptor: Option<&[std::ffi::c_void]>,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        _extra_buffer_is_reparse_point: bool,
        file_info: &mut winfsp::filesystem::OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        log::trace!("create({:?})", file_name);
        let entry = if create_options & FILE_DIRECTORY_FILE != 0 {
            FsEntry::new_directory()
        } else {
            FsEntry::new_file()
        };
        // thread::sleep(std::time::Duration::from_secs(2));
        log::info!(
            "create({}, type: {:?})",
            file_name.display(),
            entry.get_filetype()
        );

        let path = WhPath::from_fake_absolute(file_name)?;
        let name: InodeName = (&path).into();

        let itree = ITree::read_lock(&self.fs_interface.network_interface.itree, "winfsp::create")?;

        if itree.get_inode_from_path(&path).is_ok() {
            return Err(STATUS_OBJECT_NAME_EXISTS.into());
        }

        let parent = itree
            .get_inode_from_path(&path.parent().unwrap_or(WhPath::root()))
            .map_err(|_| STATUS_OBJECT_NAME_NOT_FOUND)?
            .id;

        drop(itree);
        let (inode, handle) = self
            .fs_interface
            .create(
                parent,
                name,
                entry,
                OpenFlags::from_win_u32(granted_access),
                AccessMode::from_win_u32(granted_access),
                WINDOWS_DEFAULT_PERMS_MODE,
            )
            .inspect_err(|e| log::error!("create::{e};"))?;
        *file_info.as_mut() = (&inode.meta).into();
        file_info.set_normalized_name(file_name.as_slice(), None);

        Ok(WormholeHandle {
            ino: inode.id,
            handle,
        })
    }

    fn close(&self, context: Self::FileContext) {
        log::trace!("winfsp::close({:?});", context);
        let _ = self
            .fs_interface
            .release(context.handle)
            .inspect_err(|e| log::warn!("close::{e};"));
    }

    fn cleanup(
        &self,
        context: &Self::FileContext,
        _file_name: Option<&winfsp::U16CStr>,
        flags: u32,
    ) {
        log::trace!(
            "winfsp::cleanup({:?}, {})",
            context,
            flags & FspCleanupDelete as u32 != 0
        );

        if flags & FspCleanupDelete as u32 != 0 {
            let _ = self
                .fs_interface
                .remove_inode(context.ino)
                .inspect_err(|e| log::warn!("cleanup::{e};"));
            // cannot bubble out errors here
        }
    }

    fn set_delete(
        &self,
        context: &Self::FileContext,
        _file_name: &winfsp::U16CStr,
        _delete_file: bool, // handled by winfsp
    ) -> winfsp::Result<()> {
        log::trace!("set_delete({});", context.ino);
        Ok(())
    }

    fn flush(
        &self,
        context: Option<&Self::FileContext>,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        log::trace!("flush({:?})", &context.map(|c| c.ino));
        if let Some(context) = context {
            let mut file_handles =
                FileHandleManager::write_lock(&self.fs_interface.file_handles, "release")?;

            let handle = file_handles.handles.get_mut(&context.handle);
            self.fs_interface.flush(context.ino, handle)?;
            self.get_file_info_internal(context, file_info)?;
        }
        Ok(())
    }

    fn get_file_info(
        &self,
        context: &Self::FileContext,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        log::trace!("get_file_info({})", context.ino);

        self.get_file_info_internal(context, file_info)
            .inspect_err(|e| log::warn!("get_file_info::{e};"))
    }

    fn get_security(
        &self,
        context: &Self::FileContext,
        _security_descriptor: Option<&mut [std::ffi::c_void]>, // todo: unsupported
    ) -> winfsp::Result<u64> {
        log::trace!("get_security({})", context.ino);

        Err(STATUS_INVALID_DEVICE_REQUEST.into())
    }

    // fn set_security(
    //     &self,
    //     context: &Self::FileContext,
    //     security_information: u32,
    //     modification_descriptor: winfsp::filesystem::ModificationDescriptor,
    // ) -> winfsp::Result<()> {
    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    // fn overwrite(
    //     &self,
    //     context: &Self::FileContext,
    //     file_attributes: winfsp_sys::FILE_FLAGS_AND_ATTRIBUTES,
    //     replace_file_attributes: bool,
    //     allocation_size: u64,
    //     extra_buffer: Option<&[u8]>,
    //     file_info: &mut winfsp::filesystem::FileInfo,
    // ) -> winfsp::Result<()> {
    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        _pattern: Option<&winfsp::U16CStr>, // todo: unsupported yet
        marker: winfsp::filesystem::DirMarker,
        buffer: &mut [u8],
    ) -> winfsp::Result<u32> {
        log::trace!(
            "read_directory({:?}, marker: {:?})",
            context,
            marker.inner_as_cstr().map(|s| s.to_string_lossy())
        );
        let mut entries = self
            .fs_interface
            .read_dir(context.ino)
            .inspect_err(|_| log::error!("read_directory::ERROR_NOT_FOUND"))?;

        let mut cursor = 0;

        entries.sort_by(|(_, a_name, _), (_, b_name, _)| a_name.cmp(b_name));
        let marker = match marker.inner_as_cstr() {
            Some(inner) => Some(
                inner
                    .to_string()
                    .map_err(|_| WhPathError::ConversionError {
                        source: ConversionError {},
                    })?,
            ),
            None => None,
        };

        for (_, name, meta) in entries
            .into_iter()
            .skip_while(|(_, name, _)| marker.as_ref().map(|m| *name <= *m).unwrap_or(false))
        {
            let mut dirinfo = DirInfo::<255>::default(); // !todo
                                                         // let mut info = dirinfo.file_info_mut();
            dirinfo.set_name(&name)?;
            *dirinfo.file_info_mut() = (&meta).into();
            log::trace!("dirinfo:{:?}:{:?}", &name, dirinfo.file_info_mut());
            if !dirinfo.append_to_buffer(buffer, &mut cursor) {
                break;
            }
        }
        DirInfo::<255>::finalize_buffer(buffer, &mut cursor);
        log::trace!("ok:{cursor};");
        Ok(cursor)
    }

    fn rename(
        &self,
        _context: &Self::FileContext,
        file_name: &winfsp::U16CStr,
        new_file_name: &winfsp::U16CStr,
        replace_if_exists: bool,
    ) -> winfsp::Result<()> {
        log::trace!(
            "winfsp::rename({}, {})",
            file_name.display(),
            new_file_name.display()
        );

        let path = WhPath::from_fake_absolute(file_name)?;
        let parent =
            ITree::read_lock(&self.fs_interface.network_interface.itree, "winfsp::rename")?
                .get_inode_from_path(&path.parent().unwrap_or(WhPath::root()))?
                .id;

        let new_path = WhPath::from_fake_absolute(new_file_name)?;
        let new_parent =
            ITree::read_lock(&self.fs_interface.network_interface.itree, "winfsp::rename")?
                .get_inode_from_path(&new_path.parent().unwrap_or(WhPath::root()))?
                .id;

        self.fs_interface
            .rename(
                parent,
                new_parent,
                (&path).into(),
                (&new_path).into(),
                replace_if_exists,
            )
            .inspect_err(|e| log::error!("rename: {e};"))?;
        log::trace!("ok();");
        Ok(())
    }

    fn set_basic_info(
        &self,
        context: &Self::FileContext,
        _file_attributes: u32,
        _creation_time: u64,
        last_access_time: u64,
        last_write_time: u64,
        change_time: u64,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        log::trace!("set_basic_info({})", context.ino);
        let atime = if last_access_time != 0 {
            Some(FileTime::new(last_access_time).into())
        } else {
            None
        };
        // let crtime = if creation_time != 0 {
        //     Some(
        //         FileTime::new(creation_time)
        //             .try_into()
        //             .unwrap_or_else(|_| now.clone()),
        //     )
        // } else {
        //     None
        // };
        let mtime = if last_write_time != 0 {
            Some(FileTime::new(last_write_time).into())
        } else {
            None
        };
        let ctime = if change_time != 0 {
            Some(FileTime::new(change_time).into())
        } else {
            None
        };

        self.fs_interface
            .setattr(
                context.ino,
                None,
                None,
                None,
                None,
                atime,
                mtime,
                ctime,
                Some(context.handle),
                None,
            )
            .inspect_err(|e| log::warn!("set_file_info::{e}"))?;

        self.get_file_info_internal(context, file_info)
            .inspect_err(|e| log::warn!("set_file_info::{e}"))?;
        log::trace!("ok();");
        Ok(())
    }

    fn set_file_size(
        &self,
        context: &Self::FileContext,
        new_size: u64,
        set_allocation_size: bool, // allocation is ignored;
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        log::trace!(
            "set_file_size({}, {}, {});",
            context.ino,
            new_size,
            set_allocation_size
        );
        if !set_allocation_size {
            self.fs_interface
                .setattr(
                    context.ino,
                    None,
                    None,
                    None,
                    Some(new_size),
                    None,
                    None,
                    None,
                    Some(context.handle),
                    None,
                )
                .inspect_err(|e| log::warn!("set_file_size::{e}"))?;
        }

        self.get_file_info_internal(context, file_info)
            .inspect_err(|e| log::warn!("set_file_size::{e}"))?;
        log::trace!("ok();");
        Ok(())
    }

    fn read(
        &self,
        context: &Self::FileContext,
        buffer: &mut [u8],
        offset: u64,
    ) -> winfsp::Result<u32> {
        log::trace!(
            "read({}, len: {}, offset: {})",
            context.ino,
            buffer.len(),
            offset
        );
        let size = self
            .fs_interface
            .read_file(context.ino, offset as usize, buffer, context.handle)
            .inspect_err(|e| log::warn!("read::{e}"))? as u32;
        log::trace!("ok({size});");
        Ok(size)
    }

    fn write(
        &self,
        context: &Self::FileContext,
        buffer: &[u8],
        offset: u64,
        write_to_eof: bool,
        constrained_io: bool,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<u32> {
        log::trace!(
            "write({}, len: {}, offset: {})",
            context.ino,
            buffer.len(),
            offset
        );
        let size = ITree::read_lock(&self.fs_interface.network_interface.itree, "winfsp::write")?
            .get_inode(context.ino)?
            .meta
            .size;
        let offset = if write_to_eof { size } else { offset } as usize;
        let buffer = if constrained_io {
            &buffer[0..std::cmp::min(buffer.len(), size as usize)]
        } else {
            buffer
        };
        let size = self
            .fs_interface
            .write(context.ino, buffer, offset, context.handle)
            .inspect_err(|e| log::warn!("write::{e}"))? as u32;
        self.get_file_info_internal(context, file_info)
            .inspect_err(|e| log::warn!("write::{e}"))?;
        log::trace!("ok({size});");
        Ok(size)
    }

    // fn get_dir_info_by_name(
    //     &self,
    //     context: &Self::FileContext,
    //     file_name: &winfsp::U16CStr,
    //     out_dir_info: &mut winfsp::filesystem::DirInfo,
    // ) -> winfsp::Result<()> {
    //     log::info!("get_dir_info_by_name({:?})", context);

    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    fn get_volume_info(
        &self,
        out_volume_info: &mut winfsp::filesystem::VolumeInfo,
    ) -> winfsp::Result<()> {
        log::trace!("get_volume_info");
        let info = self.fs_interface.disk.size_info()?;
        out_volume_info.free_size = info.free_size as u64;
        out_volume_info.total_size = info.total_size as u64;
        out_volume_info.set_volume_label(&*self.volume_label.read().expect("winfsp::volume_label"));
        log::trace!("ok();");
        Ok(())
    }

    fn set_volume_label(
        &self,
        volume_label: &winfsp::U16CStr,
        volume_info: &mut winfsp::filesystem::VolumeInfo,
    ) -> winfsp::Result<()> {
        log::trace!("set_volume_info");
        let info = self.fs_interface.disk.size_info()?;
        volume_info.free_size = info.free_size as u64;
        volume_info.total_size = info.total_size as u64;
        *self.volume_label.write().expect("winfsp::volume_label") = volume_label.to_string_lossy();
        volume_info.set_volume_label(&*self.volume_label.read().expect("winfsp::volume_label"));
        log::trace!("ok();");
        Ok(())
    }

    // fn get_stream_info(
    //     &self,
    //     context: &Self::FileContext,
    //     buffer: &mut [u8],
    // ) -> winfsp::Result<u32> {
    //     log::info!("get_stream_info({:?})", context);
    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    // fn get_reparse_point_by_name(
    //     &self,
    //     file_name: &winfsp::U16CStr,
    //     is_directory: bool,
    //     buffer: &mut [u8],
    // ) -> winfsp::Result<u64> {
    //     log::info!("get_reparse_point_by_name({:?})", file_name);

    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    // fn get_reparse_point(
    //     &self,
    //     context: &Self::FileContext,
    //     file_name: &winfsp::U16CStr,
    //     buffer: &mut [u8],
    // ) -> winfsp::Result<u64> {
    //     log::info!("get_reparse_point({:?})", context);

    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    // fn set_reparse_point(
    //     &self,
    //     context: &Self::FileContext,
    //     file_name: &winfsp::U16CStr,
    //     buffer: &[u8],
    // ) -> winfsp::Result<()> {
    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    // fn delete_reparse_point(
    //     &self,
    //     context: &Self::FileContext,
    //     file_name: &winfsp::U16CStr,
    //     buffer: &[u8],
    // ) -> winfsp::Result<()> {
    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    // fn get_extended_attributes(
    //     &self,
    //     context: &Self::FileContext,
    //     buffer: &mut [u8],
    // ) -> winfsp::Result<u32> {
    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    // fn set_extended_attributes(
    //     &self,
    //     context: &Self::FileContext,
    //     buffer: &[u8],
    //     file_info: &mut winfsp::filesystem::FileInfo,
    // ) -> winfsp::Result<()> {
    //     Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    // }

    fn control(
        &self,
        context: &Self::FileContext,
        _control_code: u32,
        _input: &[u8],
        _output: &mut [u8],
    ) -> winfsp::Result<u32> {
        log::trace!("control: {}", context.ino);
        Err(STATUS_INVALID_DEVICE_REQUEST.into())
    }

    fn dispatcher_stopped(&self, _normally: bool) {}
}
