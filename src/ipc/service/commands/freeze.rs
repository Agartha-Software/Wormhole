use crate::ipc::{answers::FreezeAnswer, commands::PodId, service::connection::send_answer};

pub async fn freeze<Stream>(_: PodId, stream: &mut either::Either<&mut Stream, &mut String>) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    send_answer(FreezeAnswer::Success, stream).await?;
    Ok(false)
}
