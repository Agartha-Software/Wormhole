use std::{collections::HashMap, sync::Arc};

use crate::{
    config::{types::Config, GlobalConfig, LocalConfig},
    ipc::{answers::{NewAnswer, StatusAnswer}, commands::NewRequest, service::connection::send_answer},
    network::server::Server,
    pods::{
        arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::Pod,
        whpath::JoinPath,
    },
};

pub async fn status<Stream>(
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    send_answer(StatusAnswer::Success, stream).await?;
    Ok(false)
}
