use std::collections::HashMap;

use crate::ipc::service::commands::find_pod;
use crate::ipc::{answers::InspectAnswer, commands::PodId, service::connection::send_answer};
use crate::pods::pod::Pod;

pub async fn inspect<Stream>(
    args: PodId,
    pods: &HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    match find_pod(&args, pods) {
        Some((_, pod)) => {
            send_answer(InspectAnswer::Information(pod.get_inspect_info()), stream).await?
        }
        None => send_answer(InspectAnswer::PodNotFound, stream).await?,
    };

    Ok(false)
}
