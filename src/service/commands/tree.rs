use crate::ipc::answers::TreeAnswer;
use crate::ipc::commands::PodId;
use crate::ipc::error::IoError;
use crate::pods::pod::Pod;
use crate::pods::whpath::WhPath;
use crate::service::connection::send_answer;
use crate::service::Service;

fn get_tree(pod: &Pod, path: Option<&WhPath>) -> TreeAnswer {
    let tree = pod.get_file_tree_and_hosts(path);

    match tree {
        Ok(tree) => TreeAnswer::Tree(tree.to_string()),
        Err(err) => {
            log::error!("Failed in an unexpected way when getting tree: {err}");
            TreeAnswer::PodTreeFailed(IoError {
                kind: std::io::ErrorKind::Other,
                error: err.to_string(),
            })
        }
    }
}

impl Service {
    pub async fn tree<Stream>(
        &self,
        args: PodId,
        stream: &mut either::Either<&mut Stream, &mut String>,
    ) -> std::io::Result<()>
    where
        Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
    {
        let answer = match args {
            PodId::Name(name) => {
                if let Some(pod) = self.pods.get(&name) {
                    get_tree(pod, None)
                } else {
                    TreeAnswer::PodNotFound
                }
            }
            PodId::Path(path) => {
                match self
                    .pods
                    .iter()
                    .find(|(_, pod)| path.starts_with(pod.get_mountpoint()))
                {
                    Some((_, pod)) => get_tree(
                        pod,
                        Some(
                            &WhPath::make_relative(&path, pod.get_mountpoint())
                                .map_err(|_| std::io::ErrorKind::InvalidFilename)?,
                        ),
                    ),
                    None => TreeAnswer::PodNotFound,
                }
            }
        };
        send_answer(answer, stream).await
    }
}
