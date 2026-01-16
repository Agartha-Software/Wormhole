use crate::ipc::{answers::UnfreezeAnswer, commands::PodId};
use crate::service::connection::send_answer;
use crate::service::Service;

impl Service {
    pub async fn unfreeze<Stream>(
        &mut self,
        _: PodId,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<bool>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        send_answer(UnfreezeAnswer::Success, stream).await?;
        Ok(false)
    }
}
