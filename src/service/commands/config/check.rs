use crate::{
    cli::ConfigType,
    config::{local_file::LocalConfigFile, types::Config, GlobalConfig},
    ipc::{
        answers::{CheckConfigAnswer, ConfigFileError},
        commands::PodId,
    },
    pods::{
        itree::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME},
        pod::Pod,
    },
    service::{commands::find_pod, connection::send_answer, Service},
};

pub fn get_config_from_file(
    pod: &Pod,
    config_type: &ConfigType,
) -> Result<(Option<LocalConfigFile>, Option<GlobalConfig>), ConfigFileError> {
    let mut local_path = pod.get_mountpoint().clone();
    local_path.push(LOCAL_CONFIG_FNAME);

    let mut global_path = pod.get_mountpoint().clone();
    global_path.push(GLOBAL_CONFIG_FNAME);

    match (
        local_path.exists() || !config_type.is_local(),
        global_path.exists() || !config_type.is_global(),
    ) {
        (true, true) => Ok(()),
        (false, true) => Err(ConfigFileError::MissingLocal),
        (true, false) => Err(ConfigFileError::MissingGlobal),
        (false, false) => Err(ConfigFileError::MissingBoth),
    }?;

    match (
        config_type
            .is_local()
            .then(|| LocalConfigFile::read(local_path)),
        config_type
            .is_global()
            .then(|| GlobalConfig::read(global_path)),
    ) {
        (Some(Err(local_err)), Some(Err(global_err))) => Err(ConfigFileError::InvalidBoth(
            local_err.to_string(),
            global_err.to_string(),
        )),
        (Some(Err(local_err)), _) => Err(ConfigFileError::InvalidLocal(local_err.to_string())),
        (_, Some(Err(global_err))) => Err(ConfigFileError::InvalidGlobal(global_err.to_string())),
        (local, global) => Ok((local.map(|l| l.unwrap()), global.map(|g| g.unwrap()))),
    }
}

impl Service {
    pub async fn check<Stream>(
        &self,
        args: PodId,
        config_type: ConfigType,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        match find_pod(&args, &self.pods) {
            Some((_, pod)) => match get_config_from_file(pod, &config_type) {
                Ok(_) => send_answer(CheckConfigAnswer::Success, stream).await,
                Err(err) => send_answer(CheckConfigAnswer::ConfigFileError(err), stream).await,
            },
            None => send_answer(CheckConfigAnswer::PodNotFound, stream).await,
        }
    }
}
