use crate::ipc::answers::StatusAnswer;
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
        send_answer(StatusAnswer::Success, stream).await
    }
}
