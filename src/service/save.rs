use custom_error::custom_error;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::PathBuf,
};

use crate::{
    pods::{pod::Pod, prototype::PodPrototype},
    service::Service,
};

pub fn local_data_path(socket_address: &String) -> PathBuf {
    let mut path = ProjectDirs::from("", "Agartha-Software", "Wormhole")
        .expect("Unsupported operating system, couldn't create the local data directory.")
        .data_local_dir()
        .to_path_buf();
    path.push(socket_address);
    path
}

#[derive(Deserialize, Serialize)]
struct SavedPod {
    frozen: bool,
    prototype: PodPrototype,
}

custom_error! {pub SavePodError
    LockError = "Failed to lock the pod to save",
    WriteError{ io: io::Error } = "Failed to write the file: {io}",
}

pub fn save_prototype(
    prototype: PodPrototype,
    socket_address: &String,
    frozen: bool,
) -> io::Result<()> {
    let mut path = local_data_path(socket_address);

    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    path.push(format!("{}.bak", prototype.name));

    log::trace!("Saving pod at: {path:?}");

    let saved_pod = SavedPod { frozen, prototype };

    let bin = bincode::serialize(&saved_pod).expect("Pod Prototype should always be serializable");
    let mut file = fs::File::create(path)?;
    file.write_all(&bin)
}

pub fn delete_saved_pod(socket_address: &String, name: &String) -> io::Result<()> {
    let mut path = local_data_path(socket_address);
    path.push(format!("{name}.bak"));

    if path.exists() && path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn delete_saved_pods(socket_address: &String) -> io::Result<()> {
    let folder = local_data_path(socket_address);

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

impl Service {
    pub async fn load_saved_pods(&mut self) -> io::Result<()> {
        let path = local_data_path(&self.socket);

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

            let SavedPod { frozen, prototype } = match bincode::deserialize::<SavedPod>(&bytes) {
                Ok(saved) => saved,
                Err(err) => {
                    log::trace!("Invalid Pod data found at {path:?}: {err}");
                    continue;
                }
            };

            if frozen {
                self.frozen_pods.insert(prototype.name.clone(), prototype);
            } else {
                let name = prototype.name.clone();
                match Pod::new(prototype).await {
                    Ok(pod) => self.pods.insert(name, pod),
                    Err(err) => {
                        log::trace!("Failed to create the pod '{name}': {err}");
                        // Delete failing save?
                        continue;
                    }
                };
            }
        }

        Ok(())
    }
}
