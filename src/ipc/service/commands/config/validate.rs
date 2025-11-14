use std::collections::HashMap;

use crate::{ipc::commands::PodId, pods::pod::Pod};

pub async fn validate<Stream>(
    args: PodId,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    Ok(false)
}
