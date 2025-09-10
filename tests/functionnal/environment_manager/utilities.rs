use std::{path::Path, process::ExitStatus};

use futures_util::io;
use wormhole::pods::arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME};

use crate::functionnal::environment_manager::types::{Service, CLI_BIN, MAX_PORT};

/// Returns `true` if the given services runs a pod on the given network
pub fn service_has_pod_on_network(service: &Service, network: &String) -> bool {
    service
        .pods
        .iter()
        .find(|(nw, _, _)| nw == network)
        .is_some()
}

// Returns `true` if the service is matching the requirements
pub fn service_filter(port: &Option<u16>, network: &Option<String>, service: &Service) -> bool {
    network
        .as_ref()
        .map_or_else(|| true, |nw| service_has_pod_on_network(service, &nw))
        && (port
            .as_ref()
            .map_or_else(|| true, |port| service.port == *port))
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
        .env("RUST_LOG", "wormhole=DEBUG")
        .args(args)
        .output()
        .expect("can't launch cli command");
    let stdout = std::str::from_utf8(&output.stdout).unwrap().to_string();
    let stderr = std::str::from_utf8(&output.stderr).unwrap().to_string();
    log::info!("command:\n{}", &stdout);
    log::info!("command:\n{}", &stderr);
    (output.status, stdout, stderr)
}

/// Cli commands to create a pod
pub fn cli_pod_creation_command(
    network_name: String,
    service_port: u16,
    dir_path: &Path,
    port_range: &mut std::ops::RangeFrom<u16>,
    connect_to: Option<&u16>,
) -> u16 {
    let (status, _, _) = cli_command(&[
        &format!("127.0.0.1:{service_port}"),
        "template",
        "-m",
        dir_path.to_string_lossy().to_string().as_ref(),
    ]);
    assert!(status.success(), "template cli command failed");

    let mut port;
    loop {
        port = port_range.next().unwrap();
        log::info!("trying pod on {port}");
        let (status, _, stderr) = cli_command({
            let mut args = vec![
                format!("127.0.0.1:{service_port}"),
                "new".to_string(),
                network_name.clone(),
                "-m".to_string(),
                dir_path.to_string_lossy().to_string(),
                "-p".to_string(),
                port.to_string(),
            ];

            if let Some(peer) = connect_to {
                args.push("-u".to_string());
                args.push(format!("0.0.0.0:{peer}"));
            }
            args
        });
        if status.success() {
            break;
        } else if port < MAX_PORT {
            log::error!("'new' cli command: {}", status);
            log::error!("\n{}", stderr);
        } else {
            assert_eq!(
                None,
                status.success().then_some(status),
                "'new' cli command failed and max port reached"
            );
        }

        log::trace!("Created pod on {}", port);
    }
    port
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
