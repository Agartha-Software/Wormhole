use directories::ProjectDirs;
use std::{collections::HashMap, ffi::OsStr, fs, io, path::PathBuf, sync::Arc};

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

pub async fn startup(pods: &mut HashMap<String, Pod>) -> io::Result<()> {
    let path = local_data_path();

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    for dir_entry in path.read_dir()?.filter_map(Result::ok) {
        if dir_entry.path().extension().and_then(OsStr::to_str) != Some(".bak") {
            continue;
        }

        let path = dir_entry.path();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!("Startup: Could read file at {path:?}: {err}");
                continue;
            }
        };

        let proto = match bincode::deserialize::<PodPrototype>(&bytes) {
            Ok(proto) => proto,
            Err(err) => {
                log::trace!("Startup: Invalid Pod data found at {path:?}: {err}");
                continue;
            }
        };

        let server = match Server::from_specific_address(proto.bound_socket) {
            Ok(server) => Arc::new(server),
            Err(err) => {
                log::trace!(
                    "Startup: Couldnt bind address {:?}: {err}",
                    proto.bound_socket
                );
                continue;
            }
        };

        let name = proto.name.clone();
        match Pod::new(proto, server).await {
            Ok(pod) => pods.insert(name, pod),
            Err(err) => {
                log::trace!("Startup: Failed to create the pod '{name}': {err}");
                continue;
            }
        };
    }

    Ok(())
}
