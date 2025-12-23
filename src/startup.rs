use directories::ProjectDirs;
use std::{fs, io, path::PathBuf, sync::Arc};

use crate::{network::server::Server, pods::pod::PodPrototype};

pub fn local_data_path() -> PathBuf {
    ProjectDirs::from("", "Agartha-Software", "Wormhole")
        .expect("Unsupported operating system, couldn't create the local data directory.")
        .data_local_dir()
        .to_path_buf()
}

pub fn startup() -> io::Result<()> {
    let path = local_data_path();

    log::debug!("path: {:?}", path);
    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    for dir_entry in path.read_dir()?.filter_map(Result::ok) {
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
                //log::trace!("Startup: Couldnt bind address {path:?}: {err}");
                continue;
            }
        };

        //Pod::new(proto);
    }

    Ok(())
}
