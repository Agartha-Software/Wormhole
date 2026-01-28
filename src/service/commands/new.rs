use std::net::IpAddr;

use libp2p::Multiaddr;
use port_check::{free_local_port_in_range, is_local_port_free};

use crate::{
    config::{local_file::LocalConfigFile, types::Config, GlobalConfig},
    ipc::{answers::NewAnswer, commands::NewRequest},
    pods::{
        itree::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::Pod,
        prototype::PodPrototype,
    },
    service::{connection::send_answer, Service},
};

impl Service {
    pub async fn new_command<Stream>(
        &mut self,
        args: NewRequest,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        if self.pods.contains_key(&args.name) {
            return send_answer(NewAnswer::AlreadyExist, stream).await;
        }

        if self
            .pods
            .values()
            .any(|p| *p.get_mountpoint() == args.mountpoint)
        {
            return send_answer(NewAnswer::AlreadyMounted, stream).await;
        }

        let mut global_config =
            GlobalConfig::read(args.mountpoint.join(GLOBAL_CONFIG_FNAME)).unwrap_or_default();

        let local_config =
            LocalConfigFile::read(args.mountpoint.join(LOCAL_CONFIG_FNAME)).unwrap_or_default();

        global_config = global_config.add_hosts(args.hosts);

        let (ip_type, ip) = match args.ip_address {
            Some(IpAddr::V4(v4)) => ("ip4", v4.to_string()),
            Some(IpAddr::V6(v6)) => ("ip6", v6.to_string()),
            None => ("ip4", "0.0.0.0".to_string()),
        };

        let port = match args.port {
            Some(port) => match is_local_port_free(port) {
                true => port,
                false => return send_answer(NewAnswer::PortAlreadyTaken, stream).await,
            },
            None => match free_local_port_in_range(40000..=40100) {
                Some(port) => port,
                None => return send_answer(NewAnswer::NoFreePortInRage, stream).await,
            },
        };

        let listen_address: Multiaddr = match format!("/{}/{}/tcp/{}/ws", ip_type, ip, port).parse()
        {
            Ok(listen_address) => listen_address,
            Err(err) => return send_answer(NewAnswer::InvalidIp(err.to_string()), stream).await,
        };

        let display_addr = format!("{}:{}", ip, port);

        let prototype = PodPrototype {
            global_config,
            listen_addrs: vec![listen_address.clone()],
            name: args.name.clone(),
            mountpoint: args.mountpoint,
            should_restart: local_config.restart.unwrap_or(true),
            allow_other_users: args.allow_other_users,
        };

        match Pod::new(prototype, self.nickname.clone()).await {
            Ok((pod, dialed)) => {
                self.pods.insert(args.name, pod);
                if dialed {
                    println!("New pod created successfully, listening to '{display_addr}', connected to a network.");
                } else {
                    println!("New pod created successfully, listening to '{display_addr}', no valid peers found.");
                }
                send_answer(NewAnswer::Success(display_addr, dialed), stream).await
            }
            Err(err) => send_answer(NewAnswer::FailedToCreatePod(err), stream).await,
        }
    }
}
