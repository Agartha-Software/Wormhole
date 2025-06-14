use std::{
    cmp::max,
    env::var,
    io::{self, Read, Write},
    os::{fd::AsFd, unix::net::UnixStream},
    path::Path,
    process::ExitStatus,
    time::Duration,
};

use assert_fs::TempDir;
use lazy_static::lazy_static;
use std::process::Stdio;
use wormhole::{commands::cli_commands, network::ip::IpP};

use crate::functionnal::start_log;

lazy_static! {
    pub static ref SLEEP_TIME: Duration =
        Duration::from_secs_f32(if let Ok(str_st) = var("SLEEP_TIME") {
            str_st.parse().unwrap_or(2.0)
        } else {
            2.0
        });
}

const SERVICE_MAX_PORT: u16 = 9999;
const SERVICE_MIN_PORT: u16 = 8081;
const SERVICE_BIN: &str = "./target/debug/wormholed";
const CLI_BIN: &str = "./target/debug/wormhole";

pub struct Service {
    pub instance: std::process::Child,
    #[allow(dead_code)]
    #[allow(dead_code)]
    stdin: UnixStream,
    pub ip: IpP,
    pub pods: Vec<(String, IpP, TempDir)>, // (network_name, ip, dir)
}

impl Drop for Service {
    fn drop(&mut self) {
        self.instance.stdin.take().unwrap().write(&[4]).unwrap();
        let _ = self.instance.wait();
        log::info!(
            "Stopped service {}\nStopped pods:\n{:?}",
            self.ip,
            self.pods
                .iter()
                .map(|(_, ip, _)| ip.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        );
    }
}

pub struct EnvironmentManager {
    pub services: Vec<Service>,
}

impl EnvironmentManager {
    pub fn new() -> Self {
        start_log();
        log::trace!("SLEEP_TIME for this test is {:?}", *SLEEP_TIME);
        return EnvironmentManager {
            services: Vec::new(),
        };
    }

    /// Create a service on the next available ip. No pods are created.
    pub fn add_service(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut ip = self
            .services
            .iter()
            .map(|service| &service.ip)
            .max_by(|ip1, ip2| ip1.port.cmp(&ip2.port))
            .map_or_else(
                || IpP::try_from(&format!("127.0.0.1:{SERVICE_MIN_PORT}")).unwrap(),
                |ip| {
                    let mut ip = ip.clone();
                    ip.set_port(ip.port + 1);
                    ip
                },
            );

        let (write, read) = std::os::unix::net::UnixStream::pair()?;

        // checks that no service is running on this ip
        let (mut status, _, _) = Self::cli_command(&[&ip.to_string(), "status"]);
        while status.success() {
            log::warn!(
                "\nA service is already running on {}. Trying next port...",
                ip.to_string(),
            );
            ip.set_port(ip.port + 1);
            (status, _, _) = Self::cli_command(&[&ip.to_string(), "status"]);
        }
        assert!(
            ip.port < SERVICE_MAX_PORT,
            "service port upper limit ({SERVICE_MAX_PORT}) exceeded"
        );

        let mut instance = std::process::Command::new(SERVICE_BIN)
            .args(&[ip.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        std::thread::sleep(*SLEEP_TIME);

        // checks the service viability
        let (status, out, err) = Self::cli_command(&[&ip.to_string(), "status"]);
        if !out.contains(&ip.to_string()) {
            log::error!(
                    "\nService created on {} isn't answering proper status.\n(code {}).\nCli stdout: \"{}\"\n\nCli stderr: \"{}\"\n",
                    ip.to_string(),
                    status,
                    out,
                    err,
                );

            instance.kill().unwrap();
            let _ = instance.wait(); // necessary on some os

            assert!(false, "Service {} isn't answering properly", ip.to_string());
        }

        let is_exited = instance.try_wait();
        assert!(is_exited.is_ok());
        assert!(
            is_exited.unwrap().is_none(),
            "Service {} exited unexpectedly",
            ip.to_string()
        );

        log::info!("Service started on {}", ip.to_string());
        self.services.push(Service {
            instance,
            stdin: write,
            ip: ip,
            pods: Vec::new(),
        });

        Ok(())
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
    fn cli_pod_creation_command(
        network_name: String,
        service_ip: &IpP,
        dir_path: &Path,
        ip: &IpP,
        connect_to: Option<&IpP>,
    ) -> IpP {
        let (status, _, _) = Self::cli_command(&[
            service_ip.to_string().as_ref(),
            "template",
            "-C",
            dir_path.to_string_lossy().to_string().as_ref(),
        ]);
        assert!(status.success(), "template cli command failed");

        let mut ip = ip.clone();

        loop {
            let (status, _, stderr) = Self::cli_command({
                let mut args = vec![
                    service_ip.to_string(), // service ip
                    "new".to_string(),
                    network_name.clone(), // network name
                    "-C".to_string(),
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

    /// Create pod connected to a network for each service running
    /// except if the service already has a pod on that network
    ///
    /// Pods connecting to an existing network have no guarantee on which pod they will connect
    pub fn create_network(
        &mut self,
        network_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // find the next available pod ip
        let max_pod_ip = self
            .services
            .iter()
            .map(|service| &service.pods)
            .flatten()
            .max_by(|(_, ip, _), (_, ip2, _)| ip.get_ip_last().cmp(&ip2.get_ip_last()))
            .map(|(_, ip, _)| ip.clone())
            .unwrap_or(IpP::try_from("127.0.0.1:8080").unwrap());

        // find an ip of a pod already on this network (if any)
        let conn_to = self
            .services
            .iter()
            .map(|s| &s.pods)
            .flatten()
            .find(|(nw, _, _)| *nw == network_name)
            .map(|(_, ip, _)| ip.clone());

        self.services
            .iter_mut()
            .fold((max_pod_ip, conn_to), |(max_pod_ip, conn_to), service| {
                if let Some((_, ip, _)) = service.pods.iter().find(|(nw, _, _)| *nw == network_name)
                {
                    // The service already runs a pod on this network
                    (max_pod_ip, Some(ip.clone()))
                } else {
                    // The service does not runs a pod on this network
                    let temp_dir = assert_fs::TempDir::new().expect("can't create temp dir");
                    let mut pod_ip = max_pod_ip.clone();
                    pod_ip.set_ip_last(pod_ip.get_ip_last() + 1);

                    // log::debug!("\n\n\ncreating pod for service {}", service.ip);
                    // let mut buf = Vec::new();
                    // service
                    //     .instance
                    //     .stdout
                    //     .as_mut()
                    //     .unwrap()
                    //     .take(1000)
                    //     .read(&mut buf)
                    //     .unwrap();
                    // log::debug!(
                    //     "Service stdout at this time :\n{}",
                    //     String::from_utf8_lossy(&buf)
                    // );
                    // let mut buf = Vec::new();
                    // service
                    //     .instance
                    //     .stderr
                    //     .as_mut()
                    //     .unwrap()
                    //     .take(1000)
                    //     .read(&mut buf)
                    //     .unwrap();
                    // log::debug!(
                    //     "Service stderr at this time :\n{}",
                    //     String::from_utf8_lossy(&buf)
                    // );

                    let pod_ip = Self::cli_pod_creation_command(
                        network_name.clone(),
                        &service.ip,
                        temp_dir.path(),
                        &pod_ip,
                        conn_to.as_ref(),
                    );
                    service
                        .pods
                        .push((network_name.clone(), pod_ip.clone(), temp_dir));
                    (pod_ip.clone(), Some(pod_ip))
                }
            });
        Ok(())
    }
}
