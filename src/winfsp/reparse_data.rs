use core::slice;
use std::{path::Path, ptr::addr_of_mut};

use ntapi::ntioapi::{REPARSE_DATA_BUFFER, SYMLINK_FLAG_RELATIVE};
use widestring::U16String;
use winapi::um::winnt::IO_REPARSE_TAG_SYMLINK;
use windows::Win32::Foundation::{ERROR_REPARSE_ATTRIBUTE_CONFLICT, ERROR_REPARSE_TAG_MISMATCH};
use winfsp::FspError;
use winfsp_sys::{ULONG, WCHAR};

use crate::pods::itree::{EntrySymlink, SymlinkPath};

pub struct SymbolicLinkReparseBuffer {
    substitution_name: U16String,
    print_name: U16String,
    flags: ULONG,
}

pub enum ReparseDataUnion {
    SymbolicLinkReparseBuffer(SymbolicLinkReparseBuffer),
    MountPointReparseBuffer(),
    GenericReparseBuffer(),
}

pub struct ReparseDataBuffer {
    pub reparse_data_union: ReparseDataUnion,
}

impl EntrySymlink {
    pub fn as_reparse_data_buffer(self, mountpoint: &Path) -> ReparseDataBuffer {
        let real = self.target.realize(mountpoint);
        let string = U16String::from_os_str(real.as_os_str());
        let mut flags = 0;
        if matches!(self.target, SymlinkPath::SymlinkPathRelative(_)) {
            flags |= SYMLINK_FLAG_RELATIVE;
        }
        ReparseDataBuffer {
            reparse_data_union: ReparseDataUnion::SymbolicLinkReparseBuffer(
                SymbolicLinkReparseBuffer {
                    substitution_name: string.clone(),
                    print_name: string,
                    flags,
                },
            ),
        }
    }

    pub fn from_reparse_data_buffer(
        reparse: ReparseDataBuffer,
        mountpoint: &Path,
    ) -> Result<Self, FspError> {
        match reparse.reparse_data_union {
            ReparseDataUnion::SymbolicLinkReparseBuffer(symlink) => Ok(Self::parse(
                symlink.substitution_name.to_os_string().as_ref(),
                mountpoint,
            )
            .unwrap_or_else(|e| e)),
            _ => Err(ERROR_REPARSE_TAG_MISMATCH.into()),
        }
    }
}

impl TryFrom<&REPARSE_DATA_BUFFER> for ReparseDataBuffer {
    type Error = FspError;

    fn try_from(value: &REPARSE_DATA_BUFFER) -> Result<Self, Self::Error> {
        match value.ReparseTag {
            IO_REPARSE_TAG_SYMLINK => {
                let symlink = unsafe {
                    // SAFETY: tag is checked
                    &value.u.SymbolicLinkReparseBuffer
                };
                let flags = symlink.Flags;
                let databuf = unsafe {
                    slice::from_raw_parts(
                        symlink.PathBuffer.as_ptr(),
                        value.ReparseDataLength as usize / size_of::<WCHAR>(),
                    )
                };
                log::debug!("DATABUF: {databuf:?}");
                let substitution_name = databuf
                    .get(
                        symlink.SubstituteNameOffset as usize / size_of::<WCHAR>()
                            ..(symlink.SubstituteNameLength + symlink.SubstituteNameOffset)
                                as usize
                                / size_of::<WCHAR>(),
                    )
                    .map(|str: &[u16]| str.to_vec().into());

                log::debug!("substitution_name: {substitution_name:?}");

                let print_name = databuf
                    .get(
                        symlink.PrintNameOffset as usize / size_of::<WCHAR>()
                            ..(symlink.PrintNameLength + symlink.PrintNameOffset) as usize
                                / size_of::<WCHAR>(),
                    )
                    .map(|str: &[u16]| str.to_vec().into());
                match (substitution_name, print_name) {
                    (Some(substitution_name), Some(print_name)) => Ok(Self {
                        reparse_data_union: ReparseDataUnion::SymbolicLinkReparseBuffer(
                            SymbolicLinkReparseBuffer {
                                substitution_name,
                                print_name,
                                flags,
                            },
                        ),
                    }),
                    _ => Err(ERROR_REPARSE_ATTRIBUTE_CONFLICT.into()),
                }
            }
            _ => Err(ERROR_REPARSE_TAG_MISMATCH.into()),
        }
    }
}

impl ReparseDataBuffer {
    pub fn from_buffer(buffer: &[u8]) -> Result<Self, FspError> {
        let mut backup_buff = Vec::new();
        let mut ptr = buffer.as_ptr().cast::<REPARSE_DATA_BUFFER>();
        if !ptr.is_aligned() {
            backup_buff.splice(0.., buffer);
            ptr = backup_buff.as_ptr().cast::<REPARSE_DATA_BUFFER>();
        }
        Self::try_from(unsafe { &*ptr })
    }

    pub fn as_boxed_buffer(self) -> Result<Box<[u8]>, FspError> {
        match self.reparse_data_union {
            ReparseDataUnion::SymbolicLinkReparseBuffer(symlink) => {
                // databuf layout :
                // U16Str, no null terminations
                // offset is given in bytes, not char index
                // length is given in bytes, not in chars
                // [substitution_name, print_name]
                //  ^ sub offset (0)   ^ print offset (=sub_len * sizeof(wchar))
                //  <--- sub_len*2---><print_len*2>
                let databuf = [
                    symlink.substitution_name.as_ustr().as_slice().iter(),
                    symlink.print_name.as_ustr().as_slice().iter(),
                ]
                .into_iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
                let databuf_len = (symlink.substitution_name.len() + symlink.print_name.len())
                    * size_of::<WCHAR>();

                let mut b =
                    Box::<[u8]>::new_uninit_slice(size_of::<REPARSE_DATA_BUFFER>() + databuf_len);

                let reparse_data_buffer = b.as_mut_ptr() as *mut REPARSE_DATA_BUFFER;

                unsafe {
                    addr_of_mut!((*reparse_data_buffer).ReparseDataLength)
                        .write(databuf_len as u16);
                }
                unsafe {
                    addr_of_mut!((*reparse_data_buffer).ReparseTag).write(IO_REPARSE_TAG_SYMLINK);
                }
                let symlink_out = unsafe {
                    // SAFETY: reparse_data_buffer.u is currently 0-initialized
                    addr_of_mut!((*reparse_data_buffer).u.SymbolicLinkReparseBuffer)
                };
                unsafe {
                    addr_of_mut!((*symlink_out).Flags).write(symlink.flags);
                }
                unsafe {
                    // SAFTEY: symlink_out and PathBuffer are a DST allocated with enough memory for databuf

                    addr_of_mut!((*symlink_out).PathBuffer)
                        .cast::<u16>()
                        .copy_from_nonoverlapping(databuf.as_ptr(), databuf.len());
                }
                unsafe {
                    addr_of_mut!((*symlink_out).SubstituteNameOffset).write(0);
                }
                unsafe {
                    addr_of_mut!((*symlink_out).SubstituteNameLength)
                        .write((symlink.substitution_name.len() * size_of::<WCHAR>()) as u16);
                }
                unsafe {
                    addr_of_mut!((*symlink_out).PrintNameOffset)
                        .write((symlink.substitution_name.len() * size_of::<WCHAR>()) as u16);
                }
                unsafe {
                    addr_of_mut!((*symlink_out).PrintNameLength)
                        .write((symlink.print_name.len() * size_of::<WCHAR>()) as u16);
                }
                Ok(unsafe { b.assume_init() })
            }
            ReparseDataUnion::MountPointReparseBuffer() => Err(ERROR_REPARSE_TAG_MISMATCH.into()),
            ReparseDataUnion::GenericReparseBuffer() => Err(ERROR_REPARSE_TAG_MISMATCH.into()),
        }
    }

    // fn try_from(value: &ReparseDataBuffer) -> Result<Self, Self::Error> {
    //     match value.ReparseTag {
    //         IO_REPARSE_TAG_SYMLINK => {
    //             let symlink = unsafe {
    //                 // SAFETY: tag is checked
    //                 &value.u.SymbolicLinkReparseBuffer
    //             };
    //             let flags = symlink.Flags;
    //             let substitution_name = symlink.PathBuffer.get(
    //                 symlink.SubstituteNameOffset
    //                     ..(symlink.SubstituteNameLength + symlink.SubstituteNameOffset),
    //             );
    //             let print_name = symlink.PathBuffer.get(
    //                 symlink.PrintNameOffset..(symlink.PrintNameLength + symlink.PrintNameOffset),
    //             );
    //             match (substitution_name, print_name) {
    //                 (Some(substitution_name), Some(print_name)) => Self {
    //                     reparse_data_union: ReparseDataUnion::SymbolicLinkReparseBuffer(
    //                         SymbolicLinkReparseBuffer {
    //                             substitution_name,
    //                             print_name,
    //                             flags,
    //                         },
    //                     ),
    //                 },
    //                 _ => Err(ERROR_REPARSE_ATTRIBUTE_CONFLICT.into()),
    //             }
    //         }
    //         _ => Err(ERROR_REPARSE_TAG_MISMATCH.into()),
    //     }
    // }
}
