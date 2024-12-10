use std::{
    io::ErrorKind,
    path::{PathBuf},
    sync::{Arc, RwLock},
};

use winapi::um::winnt::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_REPARSE_POINT};
use windows_permissions::constants::SecurityInformation;
use winfsp::filesystem::{FileSecurity, FileSystemContext};
use winfsp_sys::FILE_ACCESS_RIGHTS;

use crate::providers::{handle::windows::WindowsFolderHandle, FileType, Provider};

pub struct WormholeHandle(u64);

pub struct FSPController {
    pub provider: Arc<RwLock<Provider<WindowsFolderHandle>>>,
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
        if let Some(security) = reparse_point_resolver(file_name) {
            return Ok(security);
        }

        let mut descriptor_size = 0;
        match self
            .provider
            .read()
            .map_err(|_| std::io::Error::new(ErrorKind::Other, "lock poisonned"))
        {
            Ok(provider) => {
                let option_sd = if security_descriptor.is_some() {
                    Some(
                        provider
                            .folder_handle
                            .security_descriptor(SecurityInformation::all())?,
                    )
                } else {
                    None
                };
                let path = PathBuf::from(file_name.to_os_string());
                let file_type = provider
                    .index
                    .iter()
                    .filter_map(|(_, value)| if value.1 == path {Some(value.0)} else { None})
                    .nth(0);
                drop(provider);
                if let (Some(sec_dec_storage), Some(got_sd)) = (security_descriptor, option_sd) {
                    descriptor_size = unsafe {
                        winapi::um::securitybaseapi::GetSecurityDescriptorLength(
                            got_sd.as_ptr() as *mut _
                        )
                    } as usize;
                        
                    if sec_dec_storage.len() >= descriptor_size {
                        unsafe {
                            (got_sd.as_ptr() as *mut u8)
                            .copy_to(sec_dec_storage.as_ptr() as *mut u8, descriptor_size);
                        }
                    };
                }
                let mut attributes: u32 = 0;
                attributes |= match file_type.unwrap_or(FileType::Other) {
                    FileType::RegularFile => FILE_ATTRIBUTE_NORMAL,
                    FileType::Directory => FILE_ATTRIBUTE_DIRECTORY,
                    FileType::Link => FILE_ATTRIBUTE_REPARSE_POINT,
                    FileType::Other => FILE_ATTRIBUTE_READONLY, // TODO: remove ?
                };
                let sec = FileSecurity {
                    reparse: false,
                    sz_security_descriptor: descriptor_size as u64,
                    attributes,
                };
                winfsp::Result::Ok(sec)
            }
            Err(e) => winfsp::Result::Err(e.into()),
        }
    }

    fn open(
        &self,
        file_name: &winfsp::U16CStr,
        create_options: u32,
        granted_access: FILE_ACCESS_RIGHTS,
        file_info: &mut winfsp::filesystem::OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        todo!()
    }

    fn close(&self, context: Self::FileContext) {
        todo!()
    }
}
