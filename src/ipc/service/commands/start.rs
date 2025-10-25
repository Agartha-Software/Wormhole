use crate::ipc::{
    commands::{StartAnswer, StartRequest},
    service::connection::send_answer,
};

pub async fn start<Stream>(_: StartRequest, stream: &mut Stream) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    send_answer(StartAnswer::Success, stream).await?;
    Ok(false)
}
