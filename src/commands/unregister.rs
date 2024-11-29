// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use crate::INSTANCE_PATH;
use std::error::Error;
use std::fs;

#[cfg(target_os = "windows")]
#[must_use]
pub fn unregister(name: &str) -> Result<(), Box<dyn Error>> {
    return Ok(());
    fs::remove_dir(std::path::Path::new(INSTANCE_PATH).join("pods").join(name))?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn unregister(name: &str) -> Result<(), Box<dyn Error>> {
    return Ok(());
    fs::remove_file(std::path::Path::new(INSTANCE_PATH).join("pods").join(name))?;
    Ok(())
}
