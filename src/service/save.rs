use camino::{Utf8Path, Utf8PathBuf};
use custom_error::custom_error;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{
    pods::{pod::Pod, prototype::PodPrototype},
    service::Service,
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
    service_key: &ServiceKey,
    frozen: bool,
) -> io::Result<()> {
    let mut path = local_data_path(service_key);

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

impl Service {
    pub async fn load_saved_pods(&mut self) -> io::Result<()> {
        let key = ServiceKey::from_path(&self.socket);
        let path = local_data_path(&key);

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
                    Ok((pod, _)) => self.pods.insert(name, pod),
                    Err(err) => {
                        log::trace!("Failed to create the pod '{name}': {err:?}");
                        // Delete failing save?
                        continue;
                    }
                };
            }
        }

        Ok(())
    }
}
