use std::{
    os::{fd::AsFd, unix::net::UnixStream},
    path::Path,
};

use assert_fs::TempDir;
use std::process::Stdio;
use tokio::process::Command;
use wormhole::network::ip::IpP;

pub struct Service {
    pub instance: tokio::process::Child,
    #[allow(dead_code)]
    #[allow(dead_code)]
    stdin: UnixStream,
    pub ip: IpP,
    pub pods: Vec<(String, IpP, TempDir)>, // (network_name, ip, dir)
}

pub struct EnvironnementManager {
    pub services: Vec<Service>,
}

impl EnvironnementManager {
    pub fn new() -> Self {
        return EnvironnementManager {
            services: Vec::new(),
        };
    }

    fn generate_pipe(pipe_output: bool) -> Stdio {
        if pipe_output {
            Stdio::inherit()
        } else {
            Stdio::null()
        }
    }

    /// Create a service on the next available ip. No pods are created.
    pub fn add_service(&mut self, pipe_output: bool) -> Result<(), Box<dyn std::error::Error>> {
        let ip = self
            .services
            .iter()
            .map(|service| &service.ip)
            .max_by(|ip1, ip2| ip1.port.cmp(&ip2.port))
            .map_or_else(
                || IpP::try_from(&"127.0.0.1:8081".to_string()).unwrap(),
                |ip| {
                    let mut ip = ip.clone();
                    ip.set_port(ip.port + 1);
                    ip
                },
            );

        let mut command = Command::new("cargo");
        command.kill_on_drop(true);

        // GHActions does'nt pipe a stdin so we have to make one ourselves from a FD
        let (write, read) = std::os::unix::net::UnixStream::pair()?;

        let stdio = Stdio::from(read.as_fd().try_clone_to_owned().unwrap());

        let instance = command
            .args(&[
                "run".to_string(),
                "--bin".to_string(),
                "wormholed".to_string(),
                ip.to_string(),
            ])
            .stdout(Self::generate_pipe(pipe_output))
            .stderr(Self::generate_pipe(pipe_output))
            .stdin(stdio)
            .spawn()?;

        self.services.push(Service {
            instance,
            stdin: write,
            ip: ip,
            pods: Vec::new(),
        });

        Ok(())
    }

    /// Cli commands to create a pod
    fn cli_pod_creation_command(
        network_name: String,
        service_ip: &IpP,
        dir_path: &Path,
        ip: &IpP,
        connect_to: Option<&IpP>,
        pipe_output: bool,
    ) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
        let mut command = std::process::Command::new("cargo");
        log::info!("Cli template command.");
        command
            .args(&[
                "run".to_string(),
                "--bin".to_string(),
                "wormhole".to_string(),
                "template".to_string(),
                "-C".to_string(),
                dir_path.to_string_lossy().to_string(),
            ])
            .stdout(Self::generate_pipe(pipe_output))
            .stderr(Self::generate_pipe(pipe_output))
            .spawn()?
            .wait()?;

        let mut command = std::process::Command::new("cargo");
        log::info!("Cli new pod command.");
        Ok(command
            .args({
                let mut args = vec![
                    "run".to_string(),
                    "--bin".to_string(),
                    "wormhole".to_string(),
                    service_ip.to_string(), // service ip
                    "new".to_string(),
                    network_name, // network name
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
            })
            .stdout(Self::generate_pipe(pipe_output))
            .stderr(Self::generate_pipe(pipe_output))
            .spawn()?
            .wait()?)
    }

    /// Create pod connected to a network for each service running
    /// except if the service already has a pod on that network
    pub async fn create_network(
        &mut self,
        network_name: String,
        pipe_output: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("called create_network {network_name}");
        let last_pod_ip = self
            .services
            .iter()
            .map(|service| &service.pods)
            .flatten()
            .max_by(|(_, ip, _), (_, ip2, _)| ip.get_ip_last().cmp(&ip2.get_ip_last()))
            .map(|(_, ip, _)| ip.clone());

        self.services
            .iter_mut()
            .fold(last_pod_ip, |conn_to, service| {
                if let Some((_, ip, _)) = service.pods.iter().find(|(nw, _, _)| *nw == network_name)
                {
                    Some(ip.clone())
                } else {
                    let temp_dir = assert_fs::TempDir::new().expect("can't create temp dir");
                    let mut pod_ip = conn_to
                        .clone()
                        .unwrap_or(IpP::try_from(&"127.0.0.1:8080".to_string()).unwrap());
                    pod_ip.set_ip_last(pod_ip.get_ip_last() + 1);

                    println!(
                        "creating pod with parameters:\nservice: {}\npod_ip: {}\nconn_to: {:?}",
                        service.ip.to_string(),
                        pod_ip.to_string(),
                        conn_to
                    );
                    let exit_status = Self::cli_pod_creation_command(
                        network_name.clone(),
                        &service.ip,
                        temp_dir.path(),
                        &pod_ip,
                        conn_to.as_ref(),
                        pipe_output,
                    );

                    match exit_status {
                        Ok(status) if status.success() => {
                            service
                                .pods
                                .push((network_name.clone(), pod_ip.clone(), temp_dir));
                            log::info!("pod created successfully");
                            Some(pod_ip)
                        }
                        Ok(status) => panic!("Error code from the cli: {status}"),
                        Err(e) => {
                            panic!("Cli command to create pod failed: {e}");
                        }
                    }
                }
            });
        Ok(())
    }
}
