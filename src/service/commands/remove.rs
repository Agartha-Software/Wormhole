use std::collections::HashMap;

use crate::{
    ipc::{answers::RemoveAnswer, commands::RemoveRequest},
    pods::{pod::Pod, save::delete_saved_pod},
    service::{commands::find_pod, connection::send_answer},
};

pub async fn remove<Stream>(
    args: RemoveRequest,
    socket_address: &String,
    pods: &mut HashMap<String, Pod>,
    stream: &mut either::Either<&mut Stream, &mut String>,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let name = match find_pod(&args.pod, pods) {
        Some((name, _)) => name.clone(),
        None => {
            send_answer(RemoveAnswer::PodNotFound, stream).await?;
            return Ok(false);
        }
    };

    let answer = if let Some(pod) = pods.remove(&name) {
        match pod.stop().await {
            Ok(()) => match delete_saved_pod(socket_address, &name) {
                Ok(()) => RemoveAnswer::Success,
                Err(err) => RemoveAnswer::PodStopFailed(err.to_string()),
            },
            Err(err) => RemoveAnswer::PodStopFailed(err.to_string()),
        }
    } else {
        RemoveAnswer::PodNotFound
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
