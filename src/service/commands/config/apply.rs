use crate::{
    cli::ConfigType,
    ipc::{answers::ApplyConfigAnswer, commands::PodId},
    service::{
        commands::{config::check::get_config_from_file, find_pod},
        connection::send_answer,
        Service,
    },
};

impl Service {
    pub async fn apply<Stream>(
        &mut self,
        args: PodId,
        config_type: ConfigType,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        match find_pod(&args, &self.pods) {
            Some((_, pod)) => match get_config_from_file(pod, &config_type) {
                Ok((local, _global)) => {
                    if let Some(_local) = local {
                        //local.general.hostname
                        //local.general.name
                        //local.general.public_url
                        //local.general.restart
                        //pod.name = local.general.name;
                    }
                    send_answer(ApplyConfigAnswer::Success, stream).await
                }
                Err(err) => send_answer(ApplyConfigAnswer::ConfigFileError(err), stream).await,
            },
            None => send_answer(ApplyConfigAnswer::PodNotFound, stream).await,
        }
    }
}
