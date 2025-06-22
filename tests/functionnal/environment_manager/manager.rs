use std::process::Stdio;
use wormhole::network::ip::IpP;

use crate::functionnal::environment_manager::types::{StartupFiles, StopMethod};
use crate::functionnal::environment_manager::utilities::{
    cli_command, cli_pod_creation_command, copy_dir_all, service_filter,
};
use crate::functionnal::{
    environment_manager::types::{
        Service, SERVICE_BIN, SERVICE_MAX_PORT, SERVICE_MIN_PORT, SLEEP_TIME,
    },
    start_log,
};

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

        // checks that no service is running on this ip
        let (mut status, _, _) = cli_command(&[&ip.to_string(), "status"]);
        while status.success() {
            log::warn!(
                "\nA service is already running on {}. Trying next port...",
                ip.to_string(),
            );
            ip.set_port(ip.port + 1);
            (status, _, _) = cli_command(&[&ip.to_string(), "status"]);
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
        let (status, out, err) = cli_command(&[&ip.to_string(), "status"]);
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
            ip: ip,
            pods: Vec::new(),
        });

        Ok(())
    }

    pub fn remove_service(
        &mut self,
        stop_type: StopMethod,
        ip: Option<IpP>,
        network: Option<String>,
    ) {
        match stop_type {
            StopMethod::Kill => self
                .services
                .iter_mut()
                .filter(|service| service_filter(&ip, &network, *service))
                .for_each(|s| assert!(s.instance.kill().is_ok())),
            StopMethod::CtrlD => (), // just dropping will send ctrl-d
            StopMethod::CliStop => todo!(),
        }
        self.services
            .retain(|service| !service_filter(&ip, &network, service));
    }

    /// Create pod connected to a network for each service running
    /// except if the service already has a pod on that network
    ///
    /// Pods connecting to an existing network have no guarantee on which pod they will connect
    pub fn create_network(
        &mut self,
        network_name: String,
        startup_files: Option<StartupFiles>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::trace!("Creating network {network_name}");

        let mut startup_files = startup_files;

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

                    match &startup_files {
                        None => (),
                        Some(StartupFiles::ForAll(path)) => {
                            copy_dir_all(path, temp_dir.path()).unwrap()
                        }
                        Some(StartupFiles::VeryFirstOnly(path)) if conn_to.is_none() => {
                            copy_dir_all(path, temp_dir.path()).unwrap()
                        }
                        Some(StartupFiles::VeryFirstOnly(_)) => startup_files = None,
                        Some(StartupFiles::CurrentFirst(path)) => {
                            copy_dir_all(path, temp_dir.path()).unwrap();
                            startup_files = None;
                        }
                    };
                    let pod_ip = cli_pod_creation_command(
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
