use std::collections::HashMap;
use std::path::PathBuf;

use crate::ipc::answers::TreeAnswer;
use crate::ipc::error::IoError;
use crate::ipc::{commands::PodId, service::connection::send_answer};
use crate::pods::pod::Pod;

fn get_tree(pod: &Pod, path: Option<PathBuf>) -> TreeAnswer {
    let tree = pod.get_file_tree_and_hosts(path.as_deref());

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

pub async fn tree<Stream>(
    args: PodId,
    pods: &mut HashMap<String, Pod>,
    stream: &mut Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let answer = match args {
        PodId::Name(name) => {
            if let Some(pod) = pods.get(&name) {
                get_tree(pod, None)
            } else {
                TreeAnswer::PodNotFound
            }
        }
        PodId::Path(path) => {
            match pods
                .iter()
                .find(|(_, pod)| path.starts_with(pod.get_mountpoint()))
            {
                Some((_, pod)) => {
                    let local_folder_path = PathBuf::from(
                        path
                            .strip_prefix(pod.get_mountpoint())
                            .expect("Path having this prefix has been determined earlier"),
                    );
                    get_tree(pod, Some(local_folder_path))
                }
                None => TreeAnswer::PodNotFound,
            }
        }
    };
    send_answer(answer, stream).await?;
    Ok(false)
}
