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

    let port = match (local_config.general.port, args.port) {
        (None, None) => None,
        (None, Some(port)) => Some(port),
        (Some(port), None) => Some(port),
        (Some(_), Some(_)) => todo!("error conflict"),
    };

    let (server, port) = match Server::setup("0.0.0.0", port).await {
        Ok((server, port)) => (Arc::new(server), port),
        Err(answer) => {
            send_answer(answer, stream).await?;
            return Ok(false);
        }
    };

    let listen_url = match (local_config.general.url, args.listen_url) {
        (None, None) => format!("0.0.0.0:{port}"),
        (None, Some(url)) => url,
        (Some(url), None) => url,
        (Some(_), Some(_)) => todo!("error conflict"),
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
        (Some(_), Some(_)) => todo!("error conflict"),
    };

    let answer = match Pod::new(
        global_config,
        args.name.clone(),
        port,
        hostname,
        listen_url,
        args.mountpoint,
        server,
    )
    .await
    {
        Ok(pod) => {
            pods.insert(args.name, pod);
            println!("New pod created successfully at '{port}'");
            NewAnswer::Success(port)
        }
        Err(err) => NewAnswer::FailedToCreatePod(err.into()),
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
