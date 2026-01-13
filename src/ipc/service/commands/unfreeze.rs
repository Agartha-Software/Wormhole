use crate::ipc::{answers::UnfreezeAnswer, commands::PodId, service::connection::send_answer};

pub async fn unfreeze<Stream>(
    _: PodId,
    stream: &mut either::Either<&mut Stream, &mut String>,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    send_answer(UnfreezeAnswer::Success, stream).await?;
    Ok(false)
}
