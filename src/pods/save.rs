use camino::{Utf8Path, Utf8PathBuf};
use custom_error::custom_error;
use directories::ProjectDirs;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    network::server::Server,
    pods::pod::{Pod, PodPrototype},
};

/// Key representing a service, based off the socket it listens to
/// Used as the path to save known pods on shutdown
/// It may only contain a non-path string, the root /
/// and where subsequent / are replaced with '-'
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceKey(String);

impl ServiceKey {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref().to_string_lossy();
        let mut components = Utf8Path::new(&p).components().peekable();
        'prefix: loop {
            if components
                .next_if(|c| {
                    matches!(
                        c,
                        camino::Utf8Component::Prefix(_) | camino::Utf8Component::RootDir
                    )
                })
                .is_none()
            {
                break 'prefix;
            }
        }
        let mut path = Utf8PathBuf::from_iter(components)
            .into_string()
            .into_bytes();
        for c in &mut path.iter_mut() {
            if *c == b'/' || *c == b'\\' {
                *c = b'-';
            }
        }
        let path = String::from_utf8_lossy(&path);
        Self(path.into())
    }
}

impl AsRef<Path> for ServiceKey {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

pub fn local_data_path(service_key: &ServiceKey) -> PathBuf {
    let mut path = ProjectDirs::from("", "Agartha-Software", "Wormhole")
        .expect("Unsupported operating system, couldn't create the local data directory.")
        .data_local_dir()
        .to_path_buf();
    path.push(service_key);
    path
}

custom_error! {pub SavePodError
    LockError = "Failed to lock the pod to save",
    WriteError{ io: io::Error } = "Failed to write the file: {io}",
}

impl Pod {
    pub async fn save(&self, service_key: &ServiceKey) -> Result<(), SavePodError> {
        let proto = self
            .try_generate_prototype()
            .ok_or(SavePodError::LockError)?;

        let mut path = local_data_path(service_key);

        if !path.exists() {
            fs::create_dir_all(&path).map_err(|io| SavePodError::WriteError { io })?;
        }
        path.push(format!("{}.bak", proto.name));

        log::trace!("path: {path:?}");

        let bin = bincode::serialize(&proto).expect("Pod Prototype should always be serializable");
        let mut file = fs::File::create(path).map_err(|io| SavePodError::WriteError { io })?;
        file.write_all(&bin)
            .map_err(|io| SavePodError::WriteError { io })
    }
}

pub fn delete_saved_pod(service_key: &ServiceKey, name: &String) -> io::Result<()> {
    let mut path = local_data_path(service_key);
    path.push(format!("{name}.bak"));

    if path.exists() && path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn delete_saved_pods(service_key: &ServiceKey) -> io::Result<()> {
    let folder = local_data_path(service_key);

    if !folder.exists() {
        return Ok(());
    }
    for dir_entry in folder.read_dir()? {
        let path = dir_entry?.path();
        log::trace!("Deleting saved: {path:?}");
        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("bak") {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub async fn load_saved_pods(
    pods: &mut HashMap<String, Pod>,
    allow_other_users: bool,
    service_key: &ServiceKey,
) -> io::Result<()> {
    let path = local_data_path(service_key);

    if !path.exists() {
        fs::create_dir_all(&path)?;
    }

    for dir_entry in path.read_dir()?.filter_map(Result::ok) {
        let path = dir_entry.path();

        log::trace!("Loading pod backup: {:?}", path.file_stem());
        if path.extension().and_then(OsStr::to_str) != Some("bak") {
            log::trace!("Wrong extension");
            continue;
        }

        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!("Could read file at {path:?}: {err}");
                continue;
            }
        };

        let proto = match bincode::deserialize::<PodPrototype>(&bytes) {
            Ok(proto) => proto,
            Err(err) => {
                log::trace!("Invalid Pod data found at {path:?}: {err}");
                continue;
            }
        };

        let server = match Server::from_specific_address(proto.bound_socket) {
            Ok(server) => Arc::new(server),
            Err(err) => {
                log::trace!("Couldnt bind address {:?}: {err}", proto.bound_socket);
                continue;
            }
        };

        let name = proto.name.clone();
        match Pod::new(proto, allow_other_users, server).await {
            Ok(pod) => pods.insert(name, pod),
            Err(err) => {
                log::trace!("Failed to create the pod '{name}': {err}");
                continue;
            }
        };
    }

    Ok(())
}
