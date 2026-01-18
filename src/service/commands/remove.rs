use crate::{
    ipc::{answers::RemoveAnswer, commands::RemoveRequest},
    pods::save::delete_saved_pod,
    service::{commands::find_pod, connection::send_answer, Service},
};

impl Service {
    pub async fn remove<Stream>(
        &mut self,
        args: RemoveRequest,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let name = match find_pod(&args.pod, &self.pods) {
            Some((name, _)) => name.clone(),
            None => return send_answer(RemoveAnswer::PodNotFound, stream).await,
        };

        let answer = if let Some(pod) = self.pods.remove(&name) {
            match pod.stop().await {
                Ok(()) => match delete_saved_pod(&self.socket, &name) {
                    Ok(()) => RemoveAnswer::Success,
                    Err(err) => RemoveAnswer::PodStopFailed(err.to_string()),
                },
                Err(err) => RemoveAnswer::PodStopFailed(err.to_string()),
            }
        } else {
            RemoveAnswer::PodNotFound
        };
        send_answer(answer, stream).await
    }
}
