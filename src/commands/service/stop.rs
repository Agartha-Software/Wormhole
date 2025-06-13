use crate::{
    error::{CliError, CliSuccess},
    pods::pod::Pod,
};

pub async fn stop(pod: Pod) -> Result<CliSuccess, CliError> {
    match pod.stop().await {
        Ok(()) => Ok(CliSuccess::Message("Pod was stopped.".to_string())),
        Err(e) => Err(CliError::PodStopError { source: e }),
    }
}
