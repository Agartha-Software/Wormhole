use std::collections::HashMap;

use crate::{
    ipc::{
        commands::Command,
        service::commands::{freeze, new, unfreeze},
    },
    pods::pod::Pod,
};
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

pub async fn handle_connection<Stream>(pods: &mut HashMap<String, Pod>, mut stream: Stream) -> bool
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    log::debug!("Connection recieved");

    //let mut buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Command>()); TODO: Test
    let mut buffer: Vec<u8> = Vec::new();
    let _size = stream
        .read_buf(&mut buffer)
        .await
        .expect("Failed to read recieved command, shouldn't be possible!");
    match bincode::deserialize::<Command>(&buffer) {
        Ok(command) => handle_command(command, pods, stream)
            .await
            .unwrap_or_else(|e| {
                log::error!("Network Error: {e:?}"); // TODO verify relevance
                false
            }),
        Err(err) => {
            log::error!("Command recieved not recognized by the service: {err:?}");
            eprintln!("Command recieved not recognized by the service.");
            false
        }
    }
}

pub async fn send_answer<T, Stream>(answer: T, stream: &mut Stream) -> std::io::Result<()>
where
    T: Serialize,
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let serialized =
        bincode::serialize(&answer).expect("Can't serialize cli answer, shouldn't be possible!");

    stream.write_all(&serialized).await
}

async fn handle_command<Stream>(
    command: Command,
    pods: &mut HashMap<String, Pod>,
    mut stream: Stream,
) -> std::io::Result<bool>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let stream = &mut stream;

    match command {
        Command::Unfreeze(data) => unfreeze(data, stream).await,
        Command::Freeze(data) => freeze(data, stream).await,
        Command::New(data) => new(data, pods, stream).await,
    }
}

// async fn new_command<Stream>(
//     id: IdentifyPodArgs,
//     pods: &mut HashMap<String, Pod>,
//     write: WriteHalf<Stream>,
// ) where
//     Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
// {
//     if if let Some(path) = &pod_args.mountpoint {
//         pods.values().any(|p| p.get_mountpoint() == path)
//     } else {
//         false
//     } {
//         Err(CliError::Message {
//             reason: "This mount point already exist.".to_string(),
//         })
//     } else {
//         let pod_name = pod_args.name.clone();
//         match commands::service::new(pod_args).await {
//             Ok(pod) => {
//                 pods.insert(pod_name.clone(), pod);
//                 Ok(CliSuccess::WithData {
//                     message: String::from("Pod created with success"),
//                     data: pod_name,
//                 })
//             }
//             Err(e) => Err(e),
//         }
//     }
// }

//     let response_command = match command {
//         Cli::New(pod_args) => {

//         }
//         Cli::Start(pod_args) => commands::service::start(pod_args).await,
//         Cli::Stop(pod_args) => {
//             let key = pod_args
//                 .name
//                 .clone()
//                 .ok_or(CliError::PodNotFound)
//                 .or_else(|_| {
//                     pod_args
//                         .path
//                         .clone()
//                         .ok_or(CliError::InvalidArgument {
//                             arg: "missing both path and name args".to_owned(),
//                         })
//                         .and_then(|path| {
//                             pods.iter()
//                                 .find(|(_, pod)| pod.get_mountpoint() == &path)
//                                 .map(|(key, _)| key.clone())
//                                 .ok_or(CliError::PodNotFound)
//                         })
//                 });
//             match key {
//                 Err(e) => Err(e),
//                 Ok(key) => {
//                     if let Some(pod) = pods.remove(&key) {
//                         commands::service::stop(pod).await
//                     } else {
//                         Err(CliError::PodNotFound)
//                     }
//                 }
//             }
//         }
//         Cli::Remove(remove_arg) => {
//             let key = remove_arg
//                 .name
//                 .clone()
//                 .ok_or(CliError::PodNotFound)
//                 .or_else(|_| {
//                     remove_arg
//                         .path
//                         .clone()
//                         .ok_or(CliError::InvalidArgument {
//                             arg: "missing both path and name args".to_owned(),
//                         })
//                         .and_then(|path| {
//                             pods.iter()
//                                 .find(|(_, pod)| pod.get_mountpoint() == &path)
//                                 .map(|(key, _)| key.clone())
//                                 .ok_or(CliError::PodNotFound)
//                         })
//                 });
//             let pod = key.and_then(|key| pods.remove(&key).ok_or(CliError::PodNotFound));

//             match pod {
//                 Ok(pod) => commands::service::remove(remove_arg, pod).await,
//                 Err(e) => Err(e),
//             }
//         }
//         Cli::Restore(mut restore_args) => {
//             let opt_pod = if let Some(name) = &restore_args.name {
//                 pods.iter().find(|(n, _)| n == &name)
//             } else if let Some(path) = &restore_args.path {
//                 pods.iter().find(|(_, pod)| pod.get_mountpoint() == path)
//             } else {
//                 None
//             };
//             if let Some((_, pod)) = opt_pod {
//                 restore_args.path = Some(pod.get_mountpoint().clone());
//                 commands::service::restore(
//                     pod.local_config.clone(),
//                     pod.global_config.clone(),
//                     restore_args,
//                 )
//             } else {
//                 log::error!(
//                     "Pod at this path doesn't existe {:?}, {:?}",
//                     restore_args.name,
//                     restore_args.path
//                 );
//                 Err(CliError::PodRemovalFailed {
//                     name: restore_args.name.unwrap_or("".to_owned()),
//                 })
//             }
//         }
//         Cli::Apply(mut pod_conf) => {
//             // Find the good pod
//             let opt_pod = if let Some(name) = &pod_conf.name {
//                 pods.iter().find(|(n, _)| n == &name)
//             } else if let Some(path) = &pod_conf.path {
//                 pods.iter().find(|(_, pod)| pod.get_mountpoint() == path)
//             } else {
//                 None
//             };

//             //Apply new config in the pod and check if the name change
//             let res = if let Some((name, pod)) = opt_pod {
//                 pod_conf.path = Some(pod.get_mountpoint().clone());

//                 match commands::service::apply(
//                     pod.local_config.clone(),
//                     pod.global_config.clone(),
//                     pod_conf.clone(),
//                 ) {
//                     Err(err) => Err(err),
//                     Ok(_) => {
//                         match LocalConfig::read_lock(
//                             &pod.local_config.clone(),
//                             "handle_cli_command::apply",
//                         ) {
//                             Ok(local) => {
//                                 Ok(None)
//                                 // if local.general.name != *name {
//                                 //     Ok(Some((local.general.name.clone(), name.clone())))
//                                 // } else {
//                                 //     Ok(None)
//                                 // }
//                             }
//                             Err(err) => Err(CliError::WhError { source: err }),
//                         }
//                     }
//                 }
//             } else {
//                 Err(CliError::Message {
//                     reason: format!(
//                         "This name or path doesn't existe in the hashmap: {:?}, {:?}",
//                         pod_conf.name, pod_conf.path
//                     ),
//                 })
//             };

//             // Modify the name in the hashmap if it necessary
//             match res {
//                 Ok(Some((new_name, old_name))) => {
//                     let old_name: String = old_name;
//                     if let Some(pod) = pods.remove(&old_name) {
//                         pods.insert(new_name, pod);
//                         Ok(CliSuccess::Message("tt".to_owned()))
//                     } else {
//                         Err(CliError::Message {
//                             reason: "non".to_owned(),
//                         })
//                     }
//                 }
//                 Ok(None) => {
//                     todo!()
//                 }
//                 Err(err) => Err(err),
//             }
//         }
//         Cli::GetHosts(args) => {
//             if let Some((_, pod)) = if let Some(name) = &args.name {
//                 pods.iter().find(|(n, _)| n == &name)
//             } else if let Some(path) = &args.path {
//                 pods.iter().find(|(_, pod)| pod.get_mountpoint() == path)
//             } else {
//                 None
//             } {
//                 match pod.get_file_hosts(args.path.unwrap_or(".".into())) {
//                     Ok(hosts) => Ok(CliSuccess::WithData {
//                         message: "Hosts:".to_owned(),
//                         data: format!("{:?}", hosts),
//                     }),
//                     Err(error) => Err(CliError::PodInfoError { source: error }),
//                 }
//             } else {
//                 Err(CliError::PodNotFound)
//             }
//         }
//         Cli::Tree(args) => {
//             let path = args.path.and_then(|path| std::fs::canonicalize(&path).ok());
//             log::info!("TREE: canonical: {path:?}");
//             if let Some((pod, subpath)) = {
//                 if let Some(name) = &args.name {
//                     pods.iter()
//                         .find_map(|(n, pod)| (n == name).then_some((pod, None)))
//                 } else if let Some(path) = &path {
//                     pods.iter().find_map(|(_, pod)| {
//                         log::info!("TREE: pod: {:?}", &pod.get_mountpoint());
//                         path.strip_prefix(&pod.get_mountpoint())
//                             .ok()
//                             .map(|sub| (pod, Some(sub.into())))
//                     })
//                 } else {
//                     None
//                 }
//             } {
//                 match pod.get_file_tree_and_hosts(subpath) {
//                     Ok(tree) => Ok(CliSuccess::WithData {
//                         message: "File tree and hosts per file:".to_owned(),
//                         data: tree.to_string(),
//                     }),
//                     Err(error) => Err(CliError::PodInfoError { source: error }),
//                 }
//             } else {
//                 Err(CliError::PodNotFound)
//             }
//         }
//         Cli::Template(_template_arg) => todo!(),
//         Cli::Inspect => todo!(),
//         Cli::Status => Ok(CliSuccess::Message(format!("ip?"))), // todo: Rework because handle cli should be channel agnostic
//         Cli::Interrupt => todo!(),
//     };
// }
