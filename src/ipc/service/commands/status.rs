use crate::ipc::{answers::StatusAnswer, service::connection::send_answer};

pub async fn status<Stream>(stream: &mut either::Either<&mut Stream, &mut String>) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    send_answer(StatusAnswer::Success, stream).await?;
    Ok(false)
}
