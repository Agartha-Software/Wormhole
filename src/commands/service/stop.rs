use crate::{
    error::{CliError, CliSuccess},
    pods::pod::Pod,
};

pub async fn stop(pod: Pod) -> Result<CliSuccess, CliError> {
    let res = tokio::task::spawn_blocking(move || pod.stop())
        .await
        .expect("pod stop: can't spawn blocking task")
        .map(|()| "Pod was stopped.".to_string());
    match res {
        Ok(success) => Ok(CliSuccess::Message(success)),
        Err(e) => Err(CliError::PodStopError { source: e })
    }
}
