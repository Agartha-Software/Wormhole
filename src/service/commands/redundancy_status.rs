use std::collections::HashMap;

use crate::{
    ipc::{answers::RedundancyStatusAnswer, commands::PodId},
    pods::{network::redundancy::check_integrity, pod::Pod},
    service::{commands::find_pod, connection::send_answer},
};

pub async fn redundancy_status<Stream>(
    pod: PodId,
    pods: &HashMap<String, Pod>,
    stream: &mut either::Either<&mut Stream, &mut String>,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&pod, pods) {
        Some((_, pod)) => {
            let integrity = check_integrity(pod);

            send_answer(
                match integrity {
                    Ok(i) => RedundancyStatusAnswer::Status(i),
                    Err(_) => RedundancyStatusAnswer::InternalError,
                },
                stream,
            )
            .await?
        }
        None => send_answer(RedundancyStatusAnswer::PodNotFound, stream).await?,
    };

    Ok(false)
}
