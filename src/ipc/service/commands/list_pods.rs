use std::collections::HashMap;

use crate::{
    ipc::{answers::ListPodsAnswer, service::connection::send_answer},
    pods::pod::Pod,
};

pub async fn list_pods<Stream>(
    pods: &HashMap<String, Pod>,
    stream: &mut either::Either<&mut Stream, &mut String>,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let list: Vec<String> = pods.iter().map(|(name, _)| name).cloned().collect();

    send_answer(ListPodsAnswer::Pods(list), stream).await?;
    Ok(false)
}
