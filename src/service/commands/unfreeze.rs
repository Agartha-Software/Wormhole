use crate::ipc::{answers::UnfreezeAnswer, commands::PodId};
use crate::pods::pod::Pod;
use crate::service::commands::{find_frozen_pod, find_pod};
use crate::service::connection::send_answer;
use crate::service::Service;

impl Service {
    pub async fn unfreeze<Stream>(
        &mut self,
        id: PodId,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let (name, proto) = match find_frozen_pod(&id, &self.frozen_pods) {
            Some(found) => found,
            None => {
                return match find_pod(&id, &self.pods) {
                    Some(_) => send_answer(UnfreezeAnswer::AlreadyUnfrozen, stream),
                    None => send_answer(UnfreezeAnswer::PodNotFound, stream),
                }
                .await;
            }
        };

        match Pod::new(proto.clone(), self.nickname.clone()).await {
            Ok((pod, _)) => self.pods.insert(name.clone(), pod),
            Err(err) => {
                return send_answer(UnfreezeAnswer::PodCreationFailed(err), stream).await;
            }
        };

        let name = name.clone();
        self.frozen_pods
            .remove(&name)
            .expect("Already checked that the frozen_pod exist");

        send_answer(UnfreezeAnswer::Success(name), stream).await
    }
}
