use std::{
    ffi::OsString,
    fs,
    io::{Error, ErrorKind},
    sync::{Arc, RwLock},
    time::SystemTime,
};

use nt_time::FileTime;
use ntapi::ntioapi::FILE_DIRECTORY_FILE;
use winapi::shared::{
    ntstatus::STATUS_INVALID_DEVICE_REQUEST,
    winerror::{ERROR_ALREADY_EXISTS, ERROR_GEN_FAILURE},
};
use windows::Win32::Foundation::{NTSTATUS, STATUS_CANCELLED, STATUS_OBJECT_NAME_NOT_FOUND};
use winfsp::{
    filesystem::{DirInfo, FileInfo, FileSecurity, FileSystemContext, WideNameInfo},
    host::{FileSystemHost, VolumeParams},
};
use winfsp_sys::{FspCleanupDelete, FILE_ACCESS_RIGHTS};

use crate::pods::{
    arbo::{Arbo, InodeId},
    filesystem::{
        file_handle::{AccessMode, OpenFlags},
        fs_interface::{FsInterface, SimpleFileType},
    },
    whpath::WhPath,
};

pub struct WinfspHost(FileSystemHost);

#[derive(PartialEq, Debug)]
pub struct WormholeHandle {
    pub ino: InodeId,
    pub handle: u64,
}

impl std::fmt::Debug for WinfspHost {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub struct FSPController {
    pub volume_label: Arc<RwLock<String>>,
    pub fs_interface: Arc<FsInterface>,
    pub dummy_file: OsString,
    pub mount_point: WhPath,
    // pub provider: Arc<RwLock<Provider<WindowsFolderHandle>>>,
}

impl Drop for FSPController {
    fn drop(&mut self) {
        let (p, n) = self.mount_point.split_folder_file();
        let aliased = WhPath::from(&p).join(&(".".to_string() + &n));
        if fs::metadata(&aliased.inner).is_ok() {
            log::debug!(
                "moving from {} to {} ...",
                &aliased.inner,
                &self.mount_point.inner
            );
            let _ = fs::rename(&aliased.inner, &self.mount_point.inner);
        }
    }
}

impl FSPController {
    fn get_file_info_internal(
        &self,
        context: &WormholeHandle,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        let arbo = Arbo::read_lock(&self.fs_interface.arbo, "winfsp::get_file_info")?;

        match arbo.get_inode(context.ino) {
            Ok(inode) => {
                *file_info = (&inode.meta).into();
                log::trace!("ok:{:?}", file_info);
                Ok(())
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    Err(STATUS_OBJECT_NAME_NOT_FOUND.into())
                } else {
                    Err(winfsp::FspError::WIN32(ERROR_GEN_FAILURE))
                }
            }
        }
    }
}

pub fn mount_fsp(
    path: &WhPath,
    fs_interface: Arc<FsInterface>,
) -> Result<WinfspHost, std::io::Error> {
    let volume_params = VolumeParams::default();

    log::debug!("created volume params...");
    let wormhole_context = FSPController {
        volume_label: Arc::new(RwLock::new("wormhole_fs".into())),
        fs_interface,
        mount_point: path.clone(),
        dummy_file: "dummy".into(), // dummy_file: (&path.clone().rename(&("dummy_file").to_string()).inner).into(),
    };
    log::debug!("creating host...");
    let mut host = FileSystemHost::new::<FSPController>(volume_params, wormhole_context)
        .map_err(|_| std::io::Error::new(ErrorKind::Other, "oh no!"))?;
    log::debug!("created host...");

    let (p, n) = path.split_folder_file();
    let aliased = WhPath::from(&p).join(&(".".to_string() + &n));
    if fs::metadata(&path.inner).is_ok() {
        log::debug!("moving from {} to {} ...", &path.inner, &aliased.inner);
        fs::rename(&path.inner, &aliased.inner)?;
    }

    log::debug!("mounting host @ {} ...", &path.inner);
    let _ = host
        .mount(&path.inner)
        .ok()
        .ok_or(Error::other("WinFSP::mount"));
    // mount function throws the wrong error anyway so no point in inspecting it
    log::debug!("mounted host...");
    host.start_with_threads(1)?;
    log::debug!("started host...");
    Ok(WinfspHost(host))
}

impl FileSystemContext for FSPController {
    type FileContext = WormholeHandle;

    fn get_security_by_name(
        &self,
        file_name: &winfsp::U16CStr,
        security_descriptor: Option<&mut [std::ffi::c_void]>,
        reparse_point_resolver: impl FnOnce(
            &winfsp::U16CStr,
        ) -> Option<winfsp::filesystem::FileSecurity>,
    ) -> winfsp::Result<winfsp::filesystem::FileSecurity> {
        // thread::sleep(std::time::Duration::from_secs(2));
        log::trace!(
            "winfsp::get_security_by_name({}, {:?})",
            file_name.to_string_lossy(),
            security_descriptor.as_ref().map(|s| s.len())
        );

        if let Some(security) = reparse_point_resolver(file_name) {
            return Ok(security);
        }

        let path: WhPath = file_name
            .try_into()
            .inspect_err(|e| log::trace!("{}:{:?}", file_name.to_string_lossy(), e))?;

        let file_info: FileInfo =
            (&Arbo::read_lock(&self.fs_interface.arbo, "get_security_by_name")?
                .get_inode_from_path(&path)
                .inspect_err(|e| log::trace!("{}:{:?}", &path.inner, e))?
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

        let path: WhPath = file_name
            .try_into()
            .inspect_err(|e| log::warn!("open({display_name})::{:?}", e))?;
        let inode = Arbo::read_lock(&self.fs_interface.arbo, "winfsp::open")?
            .get_inode_from_path(&path)
            .inspect_err(|e| log::warn!("open({display_name})::{e};"))
            .cloned();
        match inode {
            Ok(inode) => {
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
                    handle: handle,
                })
            }
            Err(err) => {
                log::warn!("open({display_name})::{:?}", err);
                if err.kind() == ErrorKind::NotFound {
                    Err(STATUS_OBJECT_NAME_NOT_FOUND.into())
                } else {
                    Err(winfsp::FspError::WIN32(ERROR_GEN_FAILURE))
                }
            }
        }
    }

    fn close(&self, context: Self::FileContext) {
        // thread::sleep(std::time::Duration::from_secs(2));
        log::trace!("close({:?});", context);
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
        let kind = match (create_options & FILE_DIRECTORY_FILE) != 0 {
            true => SimpleFileType::Directory,
            false => SimpleFileType::File,
        };
        // thread::sleep(std::time::Duration::from_secs(2));
        log::info!("create({}, type: {:?})", file_name.display(), kind);

        let path: WhPath = file_name.try_into()?;
        let (folder, name) = path.split_folder_file();
        let arbo = Arbo::write_lock(&self.fs_interface.arbo, "winfsp::create")?;

        if let Ok(_) = arbo.get_inode_from_path(&path) {
            return Err(winfsp::FspError::WIN32(ERROR_ALREADY_EXISTS));
        }

        let parent = arbo
            .get_inode_from_path(&(&folder).into())
            .map_err(|_| STATUS_OBJECT_NAME_NOT_FOUND)?
            .id;

        drop(arbo);
        let (inode, handle) = self
            .fs_interface
            .create(
                parent,
                name,
                kind,
                OpenFlags::from_win_u32(granted_access),
                AccessMode::from_win_u32(granted_access),
                0o777, // TODO
            )
            .inspect_err(|e| log::error!("create::{e};"))?;
        *file_info.as_mut() = (&inode.meta).into();
        file_info.set_normalized_name(file_name.as_slice(), None);
        log::debug!("ok:{};", inode.id);
        Ok(WormholeHandle {
            ino: inode.id,
            handle,
        })
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
            let _ = self.fs_interface.release(context.handle)
                .inspect_err(|e| log::warn!("cleanup::{e};"));
            // cannot bubble out errors here
        }
        log::trace!("ok();");
    }

    fn flush(
        &self,
        _context: Option<&Self::FileContext>,
        _file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        Ok(())
        //         Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    }

    fn get_file_info(
        &self,
        context: &Self::FileContext,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        log::trace!("get_file_info({:?})", context);

        self.get_file_info_internal(context, file_info)
            .inspect_err(|e| log::warn!("get_file_info::{e};"))
    }

    fn get_security(
        &self,
        context: &Self::FileContext,
        _security_descriptor: Option<&mut [std::ffi::c_void]>, // todo: unsupported
    ) -> winfsp::Result<u64> {
        log::trace!("get_security({:?})", context);

        Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
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
        // thread::sleep(std::time::Duration::from_secs(2));
        log::trace!(
            "read_directory({:?}, marker: {:?})",
            context,
            marker.inner_as_cstr().map(|s| s.to_string_lossy())
        );
        let mut entries = if let Ok(entries) = self.fs_interface.read_dir(context.ino) {
            entries
        } else {
            log::error!("read_directory::ERROR_NOT_FOUND");
            return Err(STATUS_OBJECT_NAME_NOT_FOUND.into());
        };

        let mut cursor = 0;

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let marker = marker.inner_as_cstr().map(|s| s.to_string_lossy());
        for entry in entries
            .into_iter()
            .skip_while(|s| marker.as_ref().map(|m| &s.name <= m).unwrap_or(false))
        {
            let mut dirinfo = DirInfo::<255>::default(); // !todo
                                                         // let mut info = dirinfo.file_info_mut();
            dirinfo.set_name(&entry.name)?;
            *dirinfo.file_info_mut() = (&entry.meta).into();
            log::trace!("dirinfo:{}:{:?}", &entry.name, dirinfo.file_info_mut());
            if !dirinfo.append_to_buffer(buffer, &mut cursor) {
                break;
            }
        }
        DirInfo::<255>::finalize_buffer(buffer, &mut cursor);
        log::trace!("ok:{cursor};");
        Ok(cursor as u32)
    }

    fn rename(
        &self,
        _context: &Self::FileContext,
        file_name: &winfsp::U16CStr,
        new_file_name: &winfsp::U16CStr,
        replace_if_exists: bool,
    ) -> winfsp::Result<()> {
        log::info!(
            "winfsp::rename({}, {})",
            file_name.display(),
            new_file_name.display()
        );

        let path: WhPath = file_name
            .try_into()
            .inspect_err(|e| log::warn!("rename::{:?}", e))?;
        let (folder, name) = path.split_folder_file();
        let parent = Arbo::read_lock(&self.fs_interface.arbo, "winfsp::rename")?
            .get_inode_from_path(&(&folder).into())?
            .id;

        let new_path: WhPath = new_file_name
            .try_into()
            .inspect_err(|e| log::warn!("rename::{:?}", e))?;
        let (new_folder, new_name) = new_path.split_folder_file();
        let new_parent = Arbo::read_lock(&self.fs_interface.arbo, "winfsp::rename")?
            .get_inode_from_path(&(&new_folder).into())?
            .id;

        self.fs_interface
            .rename(parent, new_parent, &name, &new_name, replace_if_exists)
            .inspect_err(|e| log::error!("rename: {e};"))?;
        log::debug!("ok();");
        Ok(())
    }

    fn set_basic_info(
        &self,
        context: &Self::FileContext,
        _file_attributes: u32,
        creation_time: u64,
        last_access_time: u64,
        last_write_time: u64,
        change_time: u64,
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        log::info!("set_basic_info({:?})", context);
        let mut meta = self
            .fs_interface
            .get_inode_attributes(context.ino)
            .map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    STATUS_OBJECT_NAME_NOT_FOUND
                } else {
                    STATUS_CANCELLED
                }
            })?;
        let now = SystemTime::now();

        if last_access_time != 0 {
            meta.atime = FileTime::new(last_access_time)
                .try_into()
                .unwrap_or_else(|_| now.clone());
        }
        if creation_time != 0 {
            meta.crtime = FileTime::new(creation_time)
                .try_into()
                .unwrap_or_else(|_| now.clone());
        }
        if last_write_time != 0 {
            meta.mtime = FileTime::new(last_write_time)
                .try_into()
                .unwrap_or_else(|_| now.clone());
        }
        if change_time != 0 {
            meta.ctime = FileTime::new(change_time)
                .try_into()
                .unwrap_or_else(|_| now.clone());
        }

        self.fs_interface.set_inode_meta(context.ino, meta)?;

        self.get_file_info_internal(context, file_info)
            .inspect_err(|e| log::warn!("set_file_info::{e}"))?;
        log::debug!("ok();");
        Ok(())
    }

    fn set_delete(
        &self,
        context: &Self::FileContext,
        _file_name: &winfsp::U16CStr,
        _delete_file: bool, // handled by winfsp
    ) -> winfsp::Result<()> {
        log::trace!("set_delete({:?});", context);
        Ok(())
    }

    fn set_file_size(
        &self,
        context: &Self::FileContext,
        new_size: u64,
        _set_allocation_size: bool, // allocation is ignored;
        file_info: &mut winfsp::filesystem::FileInfo,
    ) -> winfsp::Result<()> {
        log::info!("set_file_size({:?}, {new_size})", context);
        let mut meta = self.fs_interface.get_inode_attributes(context.ino)
            .inspect_err(|e| log::error!("set_file_size::{e}"))?;
        meta.size = new_size;
        *file_info = (&meta).into();
        self.fs_interface.set_inode_meta(context.ino, meta)
            .inspect_err(|e| log::error!("set_file_size::{e}"))?;
        log::debug!("ok();");
        Ok(())
    }

    fn read(
        &self,
        context: &Self::FileContext,
        buffer: &mut [u8],
        offset: u64,
    ) -> winfsp::Result<u32> {
        log::info!("read({:?})", context);
        let size = self
            .fs_interface
            .read_file(context.ino, offset as usize, buffer)
            .inspect_err(|e| log::warn!("read::{e}"))? as u32;
        log::debug!("ok({size});");
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
        log::info!("write({:?})", context);
        let size = Arbo::read_lock(&self.fs_interface.arbo, "winfsp::write")?
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
        log::debug!("ok({size});");
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
        _context: &Self::FileContext,
        _control_code: u32,
        _input: &[u8],
        _output: &mut [u8],
    ) -> winfsp::Result<u32> {
        Err(NTSTATUS(STATUS_INVALID_DEVICE_REQUEST).into())
    }

    fn dispatcher_stopped(&self, _normally: bool) {}

    unsafe fn with_operation_response<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&mut winfsp_sys::FSP_FSCTL_TRANSACT_RSP) -> T,
    {
        unsafe {
            if let Some(context) = winfsp_sys::FspFileSystemGetOperationContext().as_ref() {
                if let Some(response) = context.Response.as_mut() {
                    return Some(f(response));
                }
            }
        }
        None
    }

    unsafe fn with_operation_request<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&winfsp_sys::FSP_FSCTL_TRANSACT_REQ) -> T,
    {
        unsafe {
            if let Some(context) = winfsp_sys::FspFileSystemGetOperationContext().as_ref() {
                if let Some(request) = context.Request.as_ref() {
                    return Some(f(request));
                }
            }
        }
        None
    }
}
