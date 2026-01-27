use crate::ipc::{answers::FreezeAnswer, commands::PodId};
use crate::service::commands::{find_frozen_pod, find_pod};
use crate::service::connection::send_answer;
use crate::service::Service;

impl Service {
    pub async fn freeze<Stream>(
        &mut self,
        id: PodId,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let (name, pod) = match find_pod(&id, &self.pods) {
            Some(found) => found,
            None => {
                return match find_frozen_pod(&id, &self.frozen_pods) {
                    Some(_) => send_answer(FreezeAnswer::AlreadyFrozen, stream),
                    None => send_answer(FreezeAnswer::PodNotFound, stream),
                }
                .await;
            }
        };

        let name = name.clone();

        match pod.try_generate_prototype() {
            Some(proto) => self.frozen_pods.insert(name.clone(), proto),
            None => {
                return send_answer(FreezeAnswer::PodBlock, stream).await;
            }
        };

        let pod = self
            .pods
            .remove(&name)
            .expect("Already checked that the pod exist");

        let answer = match pod.stop().await {
            Ok(()) => FreezeAnswer::Success(name),
            Err(err) => FreezeAnswer::PodStopFailed(err.to_string()),
        };

        send_answer(answer, stream).await
    }
}
