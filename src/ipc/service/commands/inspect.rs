use std::collections::HashMap;

use crate::ipc::answers::InspectInfo;
use crate::ipc::{answers::InspectAnswer, commands::PodId, service::connection::send_answer};
use crate::pods::pod::Pod;
use crate::pods::whpath::JoinPath;

pub async fn inspect<Stream>(
    args: PodId,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let pod_opt = match args {
        PodId::Name(name) => pods.get_key_value(&name),
        PodId::Path(path) => pods
            .iter()
            .find(|(_, pod)| pod.get_mountpoint().as_str() == path.as_str()),
    };

    match pod_opt {
        Some((host, pod)) => {
            send_answer(
                InspectAnswer::Information(InspectInfo {
                    name: host.clone(),
                    ..pod.get_inspect_info()
                }),
                stream,
            )
            .await?
        }
        None => send_answer(InspectAnswer::PodNotFound, stream).await?,
    };

    Ok(false)
}
