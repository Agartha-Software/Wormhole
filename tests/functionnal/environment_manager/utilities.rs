use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
};

use futures_util::io;
use wormhole::{
    network::ip::IpP,
    pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
};

use crate::functionnal::environment_manager::types::{Service, CLI_BIN};

/// Returns `true` if the given services runs a pod on the given network
pub fn service_has_pod_on_network(service: &Service, network: &String) -> bool {
    service
        .pods
        .iter()
        .find(|(nw, _, _)| nw == network)
        .is_some()
}

// Returns `true` if the service is matching the requirements
pub fn service_filter(ip: &Option<IpP>, network: &Option<String>, service: &Service) -> bool {
    network
        .as_ref()
        .map_or_else(|| true, |nw| service_has_pod_on_network(service, &nw))
        && (ip.as_ref().map_or_else(|| true, |ip| service.ip == *ip))
}

/// Runs a command with the cli and returns it's stdout
/// Returns (status, stdio, stderr)
pub fn cli_command<I, S>(args: I) -> (ExitStatus, String, String)
where
    I: IntoIterator<Item = S> + std::fmt::Debug,
    S: AsRef<std::ffi::OsStr>,
{
    log::trace!("Cli command with args {:?}", args);

    let output = std::process::Command::new(CLI_BIN)
        .args(args)
        .output()
        .expect("can't launch cli command");
    (
        output.status,
        std::str::from_utf8(&output.stdout).unwrap().to_string(),
        std::str::from_utf8(&output.stderr).unwrap().to_string(),
    )
}

/// Cli commands to create a pod
pub fn cli_pod_creation_command(
    network_name: String,
    service_ip: &IpP,
    dir_path: &Path,
    ip: &IpP,
    connect_to: Option<&IpP>,
) -> IpP {
    let (status, _, _) = cli_command(&[
        service_ip.to_string().as_ref(),
        "template",
        "-p",
        dir_path.to_string_lossy().to_string().as_ref(),
    ]);
    assert!(status.success(), "template cli command failed");

    let mut ip = ip.clone();

    loop {
        let (status, _, stderr) = cli_command({
            let mut args = vec![
                service_ip.to_string(),
                "new".to_string(),
                "-n".to_string(),
                network_name.clone(),
                "-p".to_string(),
                dir_path.to_string_lossy().to_string(),
                "-i".to_string(),
                ip.to_string(),
            ];

            if let Some(peer) = connect_to {
                args.push("-u".to_string());
                args.push(peer.to_string());
            }
            args
        });
        if !status.success() && !stderr.contains("AddrInUse") {
            log::error!("Cli stderr: {}", stderr);
            assert!(status.success(), "'new' cli command failed");
        } else if !status.success() {
            assert!(
                ip.get_ip_last() < 100,
                "seems that 100+ pods are already running"
            );
            log::warn!("Pod ip {} already in use", ip);
            ip.set_ip_last(ip.get_ip_last() + 1);
        } else {
            log::trace!("Created pod on {}", ip);
            break;
        }
    }
    ip
}

/// Recursively copies a directory from -> to
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Assert that all files in `dir1` exist and have the same content in `dir2`
/// Does not compare nor check config files
// taken (but edited) from https://doc.rust-lang.org/stable/nightly-rustc/src/run_make_support/assertion_helpers/mod.rs.html#113-135
pub fn assert_dirs_are_equal(dir1: impl AsRef<Path>, dir2: impl AsRef<Path>) {
    let dir2 = dir2.as_ref();

    std::fs::read_dir(dir1).unwrap().for_each(|entry| {
        let entry = entry.unwrap();
        let entry_name = entry.file_name();

        if entry_name.to_str().unwrap().contains(LOCAL_CONFIG_FNAME)
            || entry_name.to_str().unwrap().contains(GLOBAL_CONFIG_FNAME)
        {
            return;
        }

        if entry.path().is_dir() {
            assert_dirs_are_equal(&entry.path(), &dir2.join(entry_name));
        } else {
            let path2 = dir2.join(entry_name);

            let file1 =
                std::fs::read(&entry.path()).expect(&format!("{}", entry.path().to_string_lossy()));
            let file2 = std::fs::read(&path2).expect(&format!("{}", path2.to_string_lossy()));
            assert!(
                file1 == file2,
                "`{}` and `{}` have different content",
                entry.path().display(),
                path2.display(),
            );
        }
    });
}

pub fn tree_command<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S> + std::fmt::Debug,
    S: AsRef<std::ffi::OsStr>,
{
    let output = std::process::Command::new(CLI_BIN)
        .args(args)
        .output()
        .expect("can't launch tree command");

    std::str::from_utf8(&output.stdout).unwrap().to_string()
}
