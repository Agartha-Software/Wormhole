use crate::data::tree_hosts::{TreeData, TreeEntry};
use crate::error::WhError;
use crate::ipc::answers::TreeAnswer;
use crate::ipc::commands::PodId;
use crate::pods::itree::{ITreeIndex, Ino, ROOT};
use crate::pods::pod::Pod;
use crate::pods::whpath::WhPath;
use crate::service::connection::send_answer;
use std::collections::HashMap;
use std::io;

fn get_tree(pod: &Pod, path: Option<&WhPath>) -> TreeAnswer {
    let itree = pod.fs_interface.itree.read();
    let start = path
        .map(|p| itree.get_inode_from_path(p).map(|inode| inode.id))
        .unwrap_or(Ok(ROOT));

    let mut itree = {
        let owned = itree.raw_entries().clone();
        drop(itree);
        owned
    };

    let tree =
        start.and_then(|start| recurse_build_tree(&mut itree, start).ok_or(WhError::InodeNotFound));

    match tree {
        Ok(tree) => TreeAnswer::Tree(Box::new(TreeData { tree })),
        Err(err) => {
            log::error!("Failed in an unexpected way when getting tree: {err}");
            TreeAnswer::PodTreeFailed(io::Error::other(err).into())
        }
    }
}

fn recurse_build_tree(itree: &mut ITreeIndex, ino: Ino) -> Option<TreeEntry> {
    if let Some(inode) = itree.remove(&ino) {
        match &inode.entry {
            crate::pods::itree::FsEntry::Directory(children) => {
                let children = children
                    .iter()
                    .flat_map(|ino| recurse_build_tree(itree, *ino))
                    .collect();
                Some(TreeEntry::Directory(inode, children))
            }
            _ => Some(TreeEntry::File(inode)),
        }
    } else {
        None
    }
}

pub async fn tree<Stream>(
    args: PodId,
    pods: &HashMap<String, Pod>,
    stream: &mut either::Either<&mut Stream, &mut String>,
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
    send_answer(answer, stream).await?;
    Ok(false)
}
