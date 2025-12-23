use std::{collections::HashMap, sync::Arc};

use crate::{
    config::{local_file::LocalConfigFile, types::Config, GlobalConfig},
    ipc::{answers::NewAnswer, commands::NewRequest, service::connection::send_answer},
    network::server::Server,
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::{Pod, PodPrototype},
    },
};

pub async fn new<Stream>(
    args: NewRequest,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    if pods.get(&args.name).is_some() {
        send_answer(NewAnswer::AlreadyExist, stream).await?;
        return Ok(false);
    }

    if pods
        .values()
        .any(|p| *p.get_mountpoint() == args.mountpoint)
    {
        send_answer(NewAnswer::AlreadyMounted, stream).await?;
        return Ok(false);
    }

    let mut global_config =
        GlobalConfig::read(args.mountpoint.join(GLOBAL_CONFIG_FNAME)).unwrap_or_default();

    let local_config =
        LocalConfigFile::read(args.mountpoint.join(LOCAL_CONFIG_FNAME)).unwrap_or_default();

    let (server, bound_socket) = match Server::new(args.ip_address, args.port) {
        Ok((server, bound_socket)) => (Arc::new(server), bound_socket),
        Err(answer) => {
            send_answer(answer, stream).await?;
            return Ok(false);
        }
    };

    let public_url = match (local_config.general.public_url, args.public_url) {
        (None, None) => None,
        (None, Some(public_url)) => Some(public_url),
        (Some(public_url), None) => Some(public_url),
        (Some(_), Some(_)) => {
            send_answer(
                NewAnswer::ConflictWithConfig("Public url".to_string()),
                stream,
            )
            .await?;
            return Ok(false);
        }
    };
    global_config = global_config.add_hosts(public_url.clone(), args.additional_hosts);

    let hostname = match (local_config.general.hostname, args.hostname) {
        (None, None) => gethostname::gethostname()
            .into_string()
            .unwrap_or("wormhole-default-hostname".into()),
        (None, Some(hostname)) => hostname,
        (Some(hostname), None) => hostname,
        (Some(_), Some(_)) => {
            send_answer(
                NewAnswer::ConflictWithConfig("Hostname".to_string()),
                stream,
            )
            .await?;
            return Ok(false);
        }
    };

    let prototype = PodPrototype {
        global_config,
        name: args.name.clone(),
        hostname,
        public_url,
        bound_socket,
        mountpoint: args.mountpoint,
    };

    let answer = match Pod::new(prototype, server).await {
        Ok(pod) => {
            pods.insert(args.name, pod);
            println!("New pod created successfully, listening to '{bound_socket}'");
            NewAnswer::Success(bound_socket)
        }
        Err(err) => NewAnswer::FailedToCreatePod(err.into()),
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
