use crate::{
    ipc::{answers::RedundancyStatusAnswer, commands::PodId},
    pods::network::redundancy::check_integrity,
    service::{commands::find_pod, connection::send_answer, Service},
};

impl Service {
    pub async fn redundancy_status<Stream>(
        &self,
        pod: PodId,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        match find_pod(&pod, &self.pods) {
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

        Ok(())
    }
}
