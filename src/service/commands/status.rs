use crate::ipc::answers::{StatusAnswer, StatusSuccess};
use crate::service::connection::send_answer;
use crate::service::Service;

impl Service {
    pub async fn status<Stream>(
        &self,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let data = StatusSuccess {
            running: self.pods.keys().cloned().collect::<Vec<String>>(),
            frozen: self.frozen_pods.keys().cloned().collect::<Vec<String>>(),
        };
        send_answer(StatusAnswer::Success(data), stream).await
    }
}
