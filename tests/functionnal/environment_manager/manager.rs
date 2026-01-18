use crate::functionnal::environment_manager::types::{StartupFiles, StopMethod, MIN_POD_PORT};
use crate::functionnal::environment_manager::utilities::{
    cli_command, cli_pod_creation_command, copy_dir_all, service_filter,
};
use crate::functionnal::{
    environment_manager::types::{Service, MAX_SOCKET_ID, MIN_SOCKET_ID, SERVICE_BIN, SLEEP_TIME},
    start_log,
};
use std::process::Stdio;

pub struct EnvironmentManager {
    pub socket_id: std::ops::RangeFrom<u16>,
    pub pods_port: std::ops::RangeFrom<u16>,
    pub services: Vec<Service>,
}

pub fn socket_from_id(id: u16) -> String {
    format!("whtest{id}.sock")
}

impl EnvironmentManager {
    pub fn new() -> Self {
        start_log();
        log::trace!("SLEEP_TIME for this test is {:?}", *SLEEP_TIME);
        EnvironmentManager {
            socket_id: MIN_SOCKET_ID..,
            pods_port: MIN_POD_PORT..,
            services: Vec::new(),
        }
    }

    pub fn reserve_socket_id(&mut self) -> u16 {
        self.socket_id.next().expect("socket id range")
    }

    /// Create a service on the next available socket. No pods are created.
    pub fn add_service(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut socket = self.reserve_socket_id();
        log::info!("trying service on {socket}");

        // checks that no service is running on this socket
        let (mut status, _, _) = cli_command(["-s", &socket_from_id(socket), "status"]);
        while status.success() {
            log::warn!("\nA service is already running on socket {socket}. Trying next socket...");
            socket = self.reserve_socket_id();
            (status, _, _) = cli_command(["-s", &socket_from_id(socket), "status"]);
        }
        assert!(
            socket < MAX_SOCKET_ID,
            "service socket upper limit ({MAX_SOCKET_ID}) exceeded"
        );

        let mut instance = std::process::Command::new(SERVICE_BIN)
            .args(["-s", &socket_from_id(socket), "--nodeamon", "--clean"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();

        std::thread::sleep(*SLEEP_TIME);

        // checks the service viability
        let (status, _, _) = cli_command(["-s", &socket_from_id(socket), "status"]);
        if !status.success() {
            log::error!("\nCan't reach service on {}", socket_from_id(socket),);

            instance.kill().unwrap();
            let _ = instance.wait(); // necessary on some os

            panic!("Service {} isn't answering properly", socket);
        }

        let is_exited = instance.try_wait();
        assert!(is_exited.is_ok());
        assert!(
            is_exited.unwrap().is_none(),
            "Service {} exited unexpectedly",
            socket
        );

        log::info!("Service started on {}", socket);
        self.services.push(Service {
            instance,
            id: socket,
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
                .filter(|service| service_filter(&port, &network, service))
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
            .flat_map(|s| &s.pods)
            .find(|(nw, _, _)| *nw == network_name)
            .map(|(_, ip, _)| *ip);

        self.services.iter_mut().fold(conn_to, |conn_to, service| {
            if let Some((_, port, _)) = service.pods.iter().find(|(nw, _, _)| *nw == network_name) {
                // The service already runs a pod on this network
                Some(*port)
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
                    service.id,
                    temp_dir.path(),
                    &mut self.pods_port,
                    conn_to.as_ref(),
                );
                service
                    .pods
                    .push((network_name.clone(), pod_port, temp_dir));
                Some(pod_port)
            }
        });
        Ok(())
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}
