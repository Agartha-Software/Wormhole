use std::sync::Arc;

use crate::{
    config::{local_file::LocalConfigFile, types::Config, GlobalConfig},
    ipc::{answers::NewAnswer, commands::NewRequest},
    network::server::Server,
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

        let (server, bound_socket) = match Server::new(args.ip_address, args.port) {
            Ok((server, bound_socket)) => (Arc::new(server), bound_socket),
            Err(answer) => {
                return send_answer(NewAnswer::BindImpossible(answer.into()), stream).await;
            }
        };

        let public_url = match (local_config.public_url, args.public_url) {
            (None, None) => None,
            (None, Some(public_url)) => Some(public_url),
            (Some(public_url), None) if public_url.is_empty() => None,
            (Some(public_url), None) => Some(public_url),
            (Some(url_config), Some(url_args)) if url_config == url_args => Some(url_config),
            (Some(_), Some(_)) => {
                return send_answer(
                    NewAnswer::ConflictWithConfig("Public url".to_string()),
                    stream,
                )
                .await;
            }
        };

        global_config = global_config.add_hosts(args.url, args.additional_hosts);

        let hostname = match (local_config.hostname, args.hostname) {
            (None, None) => gethostname::gethostname()
                .into_string()
                .unwrap_or("wormhole-default-hostname".into()),
            (None, Some(hostname)) => hostname,
            (Some(hostname), None) => hostname,
            (Some(h_config), Some(h_args)) if h_config == h_args => h_config,
            (Some(_), Some(_)) => {
                return send_answer(
                    NewAnswer::ConflictWithConfig("Hostname".to_string()),
                    stream,
                )
                .await;
            }
        };

        let prototype = PodPrototype {
            global_config,
            name: args.name.clone(),
            hostname,
            public_url,
            bound_socket,
            mountpoint: args.mountpoint,
            should_restart: local_config.restart.unwrap_or(true),
            allow_other_users: args.allow_other_users,
        };

        match Pod::new(prototype, server).await {
            Ok(pod) => {
                self.pods.insert(args.name, pod);
                println!("New pod created successfully, listening to '{bound_socket}'");
                send_answer(NewAnswer::Success(bound_socket), stream).await
            }
            Err(err) => send_answer(NewAnswer::FailedToCreatePod(err.into()), stream).await,
        }
    }
}
