// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use crate::INSTANCE_PATH;
use std::error::Error;

#[cfg(target_os = "windows")]
#[must_use]
pub fn register(path: &std::path::PathBuf, name: &str) -> Result<(), Box<dyn Error>> {
    return Ok(());
    let canonical = path.canonicalize()?;
    std::os::windows::fs::symlink_dir(
        canonical,
        std::path::Path::new(INSTANCE_PATH).join("pods").join(name),
    )?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn register(path: &std::path::PathBuf, name: &str) -> Result<(), Box<dyn Error>> {
    return Ok(());
    let canonical = path.canonicalize()?;
    std::os::unix::fs::symlink(
        canonical,
        std::path::Path::new(INSTANCE_PATH).join("pods").join(name),
    )?;
    Ok(())
}
