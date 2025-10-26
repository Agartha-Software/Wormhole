use std::{collections::HashMap, sync::Arc};

use crate::{
    config::{types::Config, GlobalConfig, LocalConfig},
    ipc::{
        commands::{NewAnswer, NewRequest},
        service::connection::send_answer,
    },
    network::server::Server,
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::Pod,
        whpath::JoinPath,
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
    if pods
        .values()
        .any(|p| p.get_mountpoint().as_str() == args.mountpoint.as_str())
    {
        send_answer(NewAnswer::AlreadyExist, stream).await?;
    }

    let server = match Server::setup(&format!("0.0.0.0:{}", args.port)).await {
        Ok(server) => Arc::new(server),
        Err(answer) => {
            send_answer(answer, stream).await?;
            return Ok(false);
        }
    };

    let global_config = GlobalConfig::read(args.mountpoint.join(GLOBAL_CONFIG_FNAME))
        .unwrap_or_default()
        .add_hosts(args.url.unwrap_or("".to_string()), args.additional_hosts);

    let mut local_config: LocalConfig =
        LocalConfig::read(args.mountpoint.join(LOCAL_CONFIG_FNAME)).unwrap_or_default();
    local_config.general.hostname = args.hostname.unwrap_or(
        gethostname::gethostname()
            .into_string()
            .unwrap_or("wormhole-default-hostname".into()),
    );

    let answer = match Pod::new(
        global_config,
        local_config,
        args.mountpoint.as_os_str().into(),
        server,
    )
    .await
    {
        Ok(pod) => {
            pods.insert(args.name, pod);
            NewAnswer::Success
        }
        Err(err) => NewAnswer::FailedToCreatePod(err.into()),
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
