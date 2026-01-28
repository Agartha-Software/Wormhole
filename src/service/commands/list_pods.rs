use crate::{ipc::answers::ListPodsAnswer, service::{Service, connection::send_answer}};

impl Service {
    pub async fn list_pods<Stream>(
        &self,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let list: Vec<String> = self.pods.iter().map(|(name, _)| name).cloned().collect();
        
        send_answer(ListPodsAnswer::Pods(list), stream).await?;
        Ok(())
    }
}
    