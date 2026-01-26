use crate::ipc::{answers::InspectAnswer, commands::PodId};
use crate::service::commands::{find_frozen_pod, find_pod};
use crate::service::connection::send_answer;
use crate::service::Service;

impl Service {
    pub async fn inspect<Stream>(
        &self,
        id: PodId,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        match find_pod(&id, &self.pods) {
            Some((_, pod)) => {
                send_answer(InspectAnswer::Information(pod.get_inspect_info()), stream).await
            }
            None => match find_frozen_pod(&id, &self.frozen_pods) {
                Some((_, frozen)) => {
                    send_answer(
                        InspectAnswer::Information(frozen.get_inspect_info()),
                        stream,
                    )
                    .await
                }
                None => send_answer(InspectAnswer::PodNotFound, stream).await,
            },
        }
    }
}
