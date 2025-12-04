use directories::ProjectDirs;
use std::{io, path::PathBuf};

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

    let read = path.read_dir()?;

    for i in read {
        log::debug!("entry: {:?}", i);
    }

    Ok(())
}
