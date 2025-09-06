use crate::functionnal::environment_manager::types::{StartupFiles, StopMethod};
use crate::functionnal::environment_manager::utilities::{
    cli_command, cli_pod_creation_command, copy_dir_all, service_filter,
};
use crate::functionnal::{
    environment_manager::types::{Service, MAX_PORT, MIN_PORT, SERVICE_BIN, SLEEP_TIME},
    start_log,
};
use std::process::Stdio;

pub struct EnvironmentManager {
    pub port: std::ops::RangeFrom<u16>,
    pub services: Vec<Service>,
}

impl EnvironmentManager {
    pub fn new() -> Self {
        start_log();
        log::trace!("SLEEP_TIME for this test is {:?}", *SLEEP_TIME);
        return EnvironmentManager {
            port: MIN_PORT..,
            services: Vec::new(),
        };
    }

    pub fn reserve_port(&mut self) -> u16 {
        return self.port.next().expect("port range");
    }

    /// Create a service on the next available ip. No pods are created.
    pub fn add_service(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut port = self.reserve_port();
        log::info!("trying service on {port}");

        // checks that no service is running on this ip
        let (mut status, _, _) = cli_command(&[&format!("127.0.0.1:{port}"), "status"]);
        while status.success() {
            log::warn!(
                "\nA service is already running on {}. Trying next port...",
                port,
            );
            port = self.reserve_port();
            (status, _, _) = cli_command(&[&format!("127.0.0.1:{port}"), "status"]);
        }
        assert!(
            port < MAX_PORT,
            "service port upper limit ({MAX_PORT}) exceeded"
        );

        let mut instance = std::process::Command::new(SERVICE_BIN)
            .args(&[&format!("127.0.0.1:{port}")])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();

        std::thread::sleep(*SLEEP_TIME);

        // checks the service viability
        let (status, out, err) = cli_command(&[&format!("127.0.0.1:{port}"), "status"]);
        if !out.contains(&format!("127.0.0.1:{port}")) {
            log::error!(
                    "\nService created on {} isn't answering proper status.\n(code {}).\nCli stdout: \"{}\"\n\nCli stderr: \"{}\"\n",
                    port,
                    status,
                    out,
                    err,
                );

            instance.kill().unwrap();
            let _ = instance.wait(); // necessary on some os

            assert!(false, "Service {} isn't answering properly", port);
        }

        let is_exited = instance.try_wait();
        assert!(is_exited.is_ok());
        assert!(
            is_exited.unwrap().is_none(),
            "Service {} exited unexpectedly",
            port
        );

        log::info!("Service started on {}", port);
        self.services.push(Service {
            instance,
            port,
            pods: Vec::new(),
        });

        Ok(())
    }

    pub fn remove_service(
        &mut self,
        stop_type: StopMethod,
        port: Option<u16>,
        network: Option<String>,
    ) {
        match stop_type {
            StopMethod::Kill => self
                .services
                .iter_mut()
                .filter(|service| service_filter(&port, &network, *service))
                .for_each(|s| assert!(s.instance.kill().is_ok())),
            StopMethod::CtrlD => (), // just dropping will send ctrl-d
            StopMethod::CliStop => todo!(),
        }
        self.services
            .retain(|service| !service_filter(&port, &network, service));
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

        // find an ip of a pod already on this network (if any)
        let conn_to = self
            .services
            .iter()
            .map(|s| &s.pods)
            .flatten()
            .find(|(nw, _, _)| *nw == network_name)
            .map(|(_, ip, _)| ip.clone());

        self.services.iter_mut().fold(conn_to, |conn_to, service| {
            if let Some((_, port, _)) = service.pods.iter().find(|(nw, _, _)| *nw == network_name) {
                // The service already runs a pod on this network
                Some(port.clone())
            } else {
                // The service does not runs a pod on this network
                let temp_dir = assert_fs::TempDir::new().expect("can't create temp dir");

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
                let pod_port = cli_pod_creation_command(
                    network_name.clone(),
                    service.port,
                    temp_dir.path(),
                    &mut self.port,
                    conn_to.as_ref(),
                );
                service
                    .pods
                    .push((network_name.clone(), pod_port.clone(), temp_dir));
                Some(pod_port)
            }
        });
        Ok(())
    }
}
