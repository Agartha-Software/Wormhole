use std::collections::HashMap;
use std::io;

use crate::ipc::error::IoError;
use crate::ipc::{
    answers::GetHostsAnswer, commands::GetHostsRequest, service::connection::send_answer,
};
use crate::pods::pod::{Pod, PodInfoError};
use crate::pods::whpath::WhPath;

pub async fn gethosts<Stream>(
    req: GetHostsRequest,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let answer = match pods.iter().find(|(_, pod)| pod.contains(&req.path)) {
        Some((_, pod)) => match pod.get_file_hosts(
            &WhPath::make_relative(&req.path, pod.get_mountpoint())
                .map_err(|_| io::ErrorKind::InvalidFilename)?,
        ) {
            Ok(hosts) => GetHostsAnswer::Hosts(hosts),
            Err(PodInfoError::FileNotFound) => GetHostsAnswer::FileNotFound,
            Err(PodInfoError::WrongFileType { detail }) => GetHostsAnswer::WrongFileType(detail),
            Err(e) => {
                log::error!("Failed in an unexpected way on get hosts");
                GetHostsAnswer::FailedToGetHosts(IoError {
                    kind: std::io::ErrorKind::Other,
                    error: e.to_string(),
                })
            }
        },
        None => GetHostsAnswer::FileNotInsideARunningPod,
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
