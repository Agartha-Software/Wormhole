use crate::{
    ipc::answers::ListPodsAnswer,
    service::{connection::send_answer, Service},
};

impl Service {
    pub async fn list_pods<Stream>(
        &self,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let list: Vec<String> = self.pods.keys().cloned().collect();

        send_answer(ListPodsAnswer::Pods(list), stream).await?;
        Ok(())
    }
}
