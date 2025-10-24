// In rust we code
// In code we trust
// AgarthaSoftware - 2024

//use crate::INSTANCE_PATH;

use crate::error::CliResult;

#[cfg(target_os = "windows")]
#[must_use]
pub fn register(_path: &std::path::PathBuf, _name: &str) -> CliResult<()> {
    return Ok(());
    // let canonical = path.canonicalize();
    // std::os::windows::fs::symlink_dir(
    //     canonical,
    //     std::path::Path::new(INSTANCE_PATH).join("pods").join(name),
    // )?;
    // Ok(())
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn register(_path: &std::path::PathBuf, _name: &str) -> CliResult<()> {
    Ok(())
    /*
    let canonical = path.canonicalize()?;
    std::os::unix::fs::symlink(
        canonical,
        std::path::Path::new(INSTANCE_PATH).join("pods").join(name),
    )?;
    */
}
