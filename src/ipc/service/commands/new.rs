use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::{
    config::{types::Config, GlobalConfig, LocalConfig},
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

    let mut local_config: LocalConfig =
        LocalConfig::read(args.mountpoint.join(LOCAL_CONFIG_FNAME)).unwrap_or_default();
    local_config.general.hostname = args.hostname.unwrap_or(
        gethostname::gethostname()
            .into_string()
            .unwrap_or("wormhole-default-hostname".into()),
    );

    let (server, port) = match Server::setup("0.0.0.0", args.port).await {
        Ok((server, port)) => (Arc::new(server), port),
        Err(answer) => {
            send_answer(answer, stream).await?;
            return Ok(false);
        }
    };

    let answer = match Pod::new(global_config, local_config, &args.mountpoint, server).await {
        Ok(pod) => {
            pods.insert(args.name, pod);
            NewAnswer::Success(port)
        }
        Err(err) => NewAnswer::FailedToCreatePod(err.into()),
    };
    println!("New pod created successfully at '{port}'");
    send_answer(answer, stream).await?;
    Ok(false)
}
