use crate::ipc::{
    commands::{FreezeAnswer, PodId},
    service::connection::send_answer,
};

pub async fn freeze<Stream>(_: PodId, stream: &mut Stream) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    send_answer(FreezeAnswer::Success, stream).await?;
    Ok(false)
}
