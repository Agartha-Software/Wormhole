use std::io;

use crate::ipc::error::IoError;
use crate::ipc::{answers::GetHostsAnswer, commands::GetHostsRequest};
use crate::pods::pod::PodInfoError;
use crate::pods::whpath::WhPath;
use crate::service::connection::send_answer;
use crate::service::Service;

impl Service {
    pub async fn gethosts<Stream>(
        &self,
        req: GetHostsRequest,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let answer = match self.pods.iter().find(|(_, pod)| pod.contains(&req.path)) {
            Some((_, pod)) => match pod.get_file_hosts(
                &WhPath::make_relative(&req.path, pod.get_mountpoint())
                    .map_err(|_| io::ErrorKind::InvalidFilename)?,
            ) {
                Ok(hosts) => GetHostsAnswer::Hosts(hosts),
                Err(PodInfoError::FileNotFound) => GetHostsAnswer::FileNotFound,
                Err(PodInfoError::WrongFileType { detail }) => {
                    GetHostsAnswer::WrongFileType(detail)
                }
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
        send_answer(answer, stream).await
    }
}
