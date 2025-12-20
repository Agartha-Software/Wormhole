use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::{
    config::{local_file::LocalConfigFile, types::Config, GlobalConfig},
    ipc::{answers::NewAnswer, commands::NewRequest, service::connection::send_answer},
    network::server::Server,
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::Pod,
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

    let listen_address = match (local_config.general.listen_address, args.listen_address) {
        (None, None) => None,
        (None, Some(listen_address)) => Some(listen_address),
        (Some(listen_address), None) => Some(listen_address),
        (Some(_), args_address)
            if args.ip_address.is_some() || args.port.is_some() || args_address.is_some() =>
        {
            send_answer(
                NewAnswer::ConflictWithConfig("Listen address".to_string()),
                stream,
            )
            .await?;
            return Ok(false);
        }
        _ => unreachable!(),
    };

    let server = if let Some(socket_address) = listen_address {
        Server::from_socket_address(socket_address)
    } else {
        Server::from_ip_address(args.ip_address, args.port)
    };

    let (server, listen_url) = match server {
        Ok((server, listen_url)) => (Arc::new(server), listen_url),
        Err(answer) => {
            send_answer(answer, stream).await?;
            return Ok(false);
        }
    };

    if let Some(url) = args.url {
        if let Ok(socket) = url.parse::<SocketAddr>() {
            //For now this socket addr is only used as a way to verify the host but then it could be used futher
            global_config = global_config.add_hosts(Some(socket), args.additional_hosts);
        } else {
            send_answer(NewAnswer::InvalidUrlIp, stream).await?;
            return Ok(false);
        }
    } else {
        global_config = global_config.add_hosts(None, args.additional_hosts);
    }

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

    let answer = match Pod::new(
        global_config,
        args.name.clone(),
        hostname,
        listen_url.clone(),
        args.mountpoint,
        server,
    )
    .await
    {
        Ok(pod) => {
            pods.insert(args.name, pod);
            println!("New pod created successfully, listening to '{listen_url}'");
            NewAnswer::Success(listen_url)
        }
        Err(err) => NewAnswer::FailedToCreatePod(err.into()),
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
