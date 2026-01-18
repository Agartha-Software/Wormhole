use std::sync::Arc;

use crate::ipc::answers::RestartAnswer;
use crate::ipc::commands::PodId;
use crate::network::server::Server;
use crate::pods::pod::Pod;
use crate::service::commands::{find_frozen_pod, find_pod};
use crate::service::connection::send_answer;
use crate::service::Service;

impl Service {
    pub async fn restart<Stream>(
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
                    Some(_) => send_answer(RestartAnswer::PodFrozen, stream),
                    None => send_answer(RestartAnswer::PodNotFound, stream),
                }
                .await;
            }
        };

        let name = name.clone();

        let proto = match pod.try_generate_prototype() {
            Some(proto) => proto,
            None => {
                return send_answer(RestartAnswer::PodBlock, stream).await;
            }
        };

        let pod = self
            .pods
            .remove(&name)
            .expect("Already checked that the pod exist");

        if let Err(err) = pod.stop().await {
            RestartAnswer::PodStopFailed(err.to_string());
        }

        let server = match Server::from_specific_address(proto.bound_socket) {
            Ok(server) => Arc::new(server),
            Err(err) => {
                return send_answer(RestartAnswer::CouldntBind(err.into()), stream).await;
            }
        };

        match Pod::new(proto, server).await {
            Ok(pod) => {
                self.pods.insert(name.clone(), pod);
                send_answer(RestartAnswer::Success(name), stream).await
            }
            Err(err) => send_answer(RestartAnswer::PodCreationFailed(err.into()), stream).await,
        }
    }
}
