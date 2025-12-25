use custom_error::custom_error;
use directories::ProjectDirs;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};

use crate::{
    network::server::Server,
    pods::pod::{Pod, PodPrototype},
};

pub fn local_data_path() -> PathBuf {
    ProjectDirs::from("", "Agartha-Software", "Wormhole")
        .expect("Unsupported operating system, couldn't create the local data directory.")
        .data_local_dir()
        .to_path_buf()
}

custom_error! {pub SavePodError
    LockError = "Failed to lock the pod to save",
    WriteError{ io: io::Error } = "Failed to write the file: {io}",
}

impl Pod {
    pub async fn save_pod(&self) -> Result<(), SavePodError> {
        let proto = self
            .try_generate_prototype()
            .ok_or(SavePodError::LockError)?;

        let mut path = local_data_path();

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

pub fn delete_saved_pods() -> io::Result<()> {
    for dir_entry in local_data_path().read_dir()? {
        let path = dir_entry?.path();
        if path.is_file() && path.extension().and_then(OsStr::to_str) == Some("bak") {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub async fn load_saved_pods(pods: &mut HashMap<String, Pod>) -> io::Result<()> {
    let path = local_data_path();

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

        let path = path;
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
        match Pod::new(proto, server).await {
            Ok(pod) => pods.insert(name, pod),
            Err(err) => {
                log::trace!("Failed to create the pod '{name}': {err}");
                continue;
            }
        };
    }

    Ok(())
}
