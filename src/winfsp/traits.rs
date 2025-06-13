use crate::{
    error::WhError,
    pods::{
        arbo::Metadata,
        filesystem::{
            attrs::SetAttrError,
            file_handle::{AccessMode, OpenFlags},
            fs_interface::SimpleFileType,
            make_inode::{CreateError, MakeInodeError},
            open::OpenError,
            read::ReadError,
            rename::RenameError,
            write::WriteError,
        },
        network::pull_file::PullError,
        whpath::WhPath,
    },
};
use nt_time::FileTime;
use windows::Win32::{
    Foundation::{
        GENERIC_EXECUTE, GENERIC_READ, GENERIC_WRITE, NTSTATUS, STATUS_ACCESS_DENIED,
        STATUS_DATA_ERROR, STATUS_DIRECTORY_NOT_EMPTY, STATUS_FILE_IS_A_DIRECTORY,
        STATUS_INVALID_HANDLE, STATUS_INVALID_PARAMETER, STATUS_NETWORK_UNREACHABLE,
        STATUS_NOT_A_DIRECTORY, STATUS_OBJECT_NAME_EXISTS, STATUS_OBJECT_NAME_INVALID,
        STATUS_OBJECT_NAME_NOT_FOUND, STATUS_OBJECT_PATH_NOT_FOUND, STATUS_PENDING,
        STATUS_POSSIBLE_DEADLOCK,
    },
    Storage::FileSystem::{FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_DIRECTORY, SYNCHRONIZE},
};
use winfsp::{filesystem::FileInfo, FspError};

impl TryFrom<&winfsp::U16CStr> for WhPath {
    type Error = NTSTATUS;

    fn try_from(value: &winfsp::U16CStr) -> Result<WhPath, Self::Error> {
        match value.to_string() {
            Err(_) => Err(STATUS_OBJECT_NAME_INVALID),
            Ok(string) => Ok(WhPath::from(&string.replace("\\", "/"))),
        }
    }
}

impl WhPath {
    pub fn to_winfsp(&self) -> String {
        self.inner.replace("/", "\\")
    }
}

impl From<Metadata> for FileInfo {
    fn from(value: Metadata) -> Self {
        (&value).into()
    }
}

impl From<&Metadata> for FileInfo {
    fn from(value: &Metadata) -> Self {
        let attributes = match value.kind {
            SimpleFileType::File => FILE_ATTRIBUTE_ARCHIVE,
            SimpleFileType::Directory => FILE_ATTRIBUTE_DIRECTORY,
        };
        let now = FileTime::now();
        FileInfo {
            file_attributes: attributes.0,
            reparse_tag: 0,
            allocation_size: value.size as u64,
            file_size: value.size as u64,
            creation_time: FileTime::try_from(value.crtime).unwrap_or(now).to_raw(),
            last_access_time: FileTime::try_from(value.atime).unwrap_or(now).to_raw(),
            last_write_time: FileTime::try_from(value.mtime).unwrap_or(now).to_raw(),
            change_time: FileTime::try_from(value.ctime).unwrap_or(now).to_raw(),
            index_number: value.ino,
            hard_links: 0,
            ea_size: 0,
        }
    }
}

impl From<WhError> for FspError {
    fn from(value: WhError) -> Self {
        match value {
            WhError::InodeNotFound => STATUS_OBJECT_NAME_NOT_FOUND.into(),
            WhError::InodeIsNotADirectory => STATUS_NOT_A_DIRECTORY.into(),
            WhError::DeadLock => STATUS_POSSIBLE_DEADLOCK.into(),
            WhError::NetworkDied { called_from: _ } => STATUS_NETWORK_UNREACHABLE.into(),
            WhError::WouldBlock { called_from: _ } => STATUS_PENDING.into(),
            WhError::InodeIsADirectory => STATUS_FILE_IS_A_DIRECTORY.into(),
        }
    }
}

impl From<MakeInodeError> for FspError {
    fn from(value: MakeInodeError) -> Self {
        match value {
            MakeInodeError::AlreadyExist => STATUS_OBJECT_NAME_EXISTS.into(),
            MakeInodeError::LocalCreationFailed { io } => io.into(),
            MakeInodeError::ParentNotFolder => STATUS_NOT_A_DIRECTORY.into(),
            MakeInodeError::ParentNotFound => STATUS_OBJECT_NAME_NOT_FOUND.into(),
            MakeInodeError::WhError { source } => source.into(),
            MakeInodeError::ProtectedNameIsFolder => STATUS_NOT_A_DIRECTORY.into(),
        }
    }
}

impl From<PullError> for FspError {
    fn from(value: PullError) -> Self {
        match value {
            PullError::WhError { source } => source.into(),
            PullError::NoHostAvailable => STATUS_DATA_ERROR.into(),
        }
    }
}

impl From<ReadError> for FspError {
    fn from(value: ReadError) -> Self {
        match value {
            ReadError::WhError { source } => source.into(),
            ReadError::PullError { source } => source.into(),
            ReadError::LocalReadFailed { io } => io.into(),
            ReadError::CantPull => STATUS_NETWORK_UNREACHABLE.into(),
            ReadError::NoReadPermission => STATUS_ACCESS_DENIED.into(),
            ReadError::NoFileHandle => STATUS_INVALID_HANDLE.into(),
        }
    }
}

impl From<WriteError> for FspError {
    fn from(value: WriteError) -> Self {
        match value {
            WriteError::WhError { source } => source.into(),
            WriteError::LocalWriteFailed { io } => io.into(),
            WriteError::NoFileHandle => STATUS_INVALID_HANDLE.into(),
            WriteError::NoWritePermission => STATUS_ACCESS_DENIED.into(),
        }
    }
}

impl From<RenameError> for FspError {
    fn from(value: RenameError) -> Self {
        match value {
            RenameError::WhError { source } => source.into(),
            RenameError::OverwriteNonEmpty => STATUS_DIRECTORY_NOT_EMPTY.into(),
            RenameError::LocalOverwriteFailed { io } => io.into(),
            RenameError::SourceParentNotFound => STATUS_OBJECT_PATH_NOT_FOUND.into(),
            RenameError::SourceParentNotFolder => STATUS_NOT_A_DIRECTORY.into(),
            RenameError::DestinationParentNotFound => STATUS_OBJECT_PATH_NOT_FOUND.into(),
            RenameError::DestinationParentNotFolder => STATUS_NOT_A_DIRECTORY.into(),
            RenameError::DestinationExists => STATUS_OBJECT_NAME_EXISTS.into(),
            RenameError::LocalRenamingFailed { io } => io.into(),
            RenameError::ProtectedNameIsFolder => STATUS_FILE_IS_A_DIRECTORY.into(),
            RenameError::ReadFailed { source } => source.into(),
            RenameError::LocalWriteFailed { io } => io.into(),
        }
    }
}

impl From<OpenError> for FspError {
    fn from(value: OpenError) -> Self {
        match value {
            OpenError::WhError { source } => source.into(),
            OpenError::TruncReadOnly => STATUS_ACCESS_DENIED.into(),
            OpenError::WrongPermissions => STATUS_ACCESS_DENIED.into(),
            OpenError::MultipleAccessFlags => STATUS_INVALID_PARAMETER.into(),
        }
    }
}

impl From<CreateError> for FspError {
    fn from(value: CreateError) -> Self {
        match value {
            CreateError::WhError { source } => source.into(),
            CreateError::MakeInode { source } => source.into(),
            CreateError::OpenError { source } => source.into(),
        }
    }
}

impl From<SetAttrError> for FspError {
    fn from(value: SetAttrError) -> Self {
        match value {
            SetAttrError::WhError { source } => source.into(),
            SetAttrError::SizeNoPerm => STATUS_ACCESS_DENIED.into(),
            SetAttrError::InvalidFileHandle => STATUS_INVALID_HANDLE.into(),
            SetAttrError::SetFileSizeIoError { io } => io.into(),
            SetAttrError::SetPermIoError { io } => io.into(),
        }
    }
}

impl AccessMode {
    pub fn from_win_u32(access: u32) -> AccessMode {
        if access & (GENERIC_READ.0 & !SYNCHRONIZE.0) != 0
            && access & (GENERIC_WRITE.0 & !SYNCHRONIZE.0) != 0
        {
            return AccessMode::ReadWrite;
        }
        if access & (GENERIC_WRITE.0 & !SYNCHRONIZE.0) != 0 {
            return AccessMode::Write;
        }
        if access & (GENERIC_EXECUTE.0 & !SYNCHRONIZE.0) != 0 {
            return AccessMode::Execute;
        }
        if access & (GENERIC_READ.0 & !SYNCHRONIZE.0) != 0 {
            return AccessMode::Read;
        }
        return AccessMode::Void;
    }
}

impl OpenFlags {
    pub fn from_win_u32(access: u32) -> OpenFlags {
        OpenFlags {
            no_atime: false,
            direct: false,
            trunc: false,
            exec: access & (GENERIC_EXECUTE.0 & !SYNCHRONIZE.0) != 0,
        }
    }
}
