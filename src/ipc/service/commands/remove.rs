use std::collections::HashMap;

use crate::{
    ipc::{
        answers::RemoveAnswer,
        commands::RemoveRequest,
        service::{commands::find_pod, connection::send_answer},
    },
    pods::pod::Pod,
};

pub async fn remove<Stream>(
    args: RemoveRequest,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let name = match find_pod(args.pod, pods) {
        Some((name, _)) => name.clone(),
        None => {
            send_answer(RemoveAnswer::PodNotFound, stream).await?;
            return Ok(false);
        }
    };

    let answer = if let Some(pod) = pods.remove(&name) {
        match pod.stop().await {
            Ok(()) => RemoveAnswer::Success,
            Err(e) => RemoveAnswer::PodStopFailed(e.to_string()),
        }
    } else {
        RemoveAnswer::PodNotFound
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
