use crate::{
    ipc::{answers::StatsPerFiletypeAnswer, commands::PodId},
    service::{Service, commands::find_pod, connection::send_answer},
};

impl Service {
    pub async fn stats_per_filetype<Stream>(
        &self,
        pod: PodId,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        match find_pod(&pod, &self.pods) {
            Some((_, pod)) => {
                let stats = pod.get_stats_per_filetype();
                
                send_answer(
                    match stats {
                        Ok(i) => StatsPerFiletypeAnswer::Stats(i),
                        Err(_) => StatsPerFiletypeAnswer::InternalError,
                    },
                    stream,
                )
                .await?
            }
            None => send_answer(StatsPerFiletypeAnswer::PodNotFound, stream).await?,
        };
        
        Ok(())
    }
}
    