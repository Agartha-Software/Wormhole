// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use std::collections::HashMap;
/**DOC
 * Important variables to know :
 * nfa_rx - nfa_tx
 *  Use nfa_tx to send a file related message to the newtork_file_actions function
 *
 * Important functions to know :
 *
 * local_cli_watchdog
 *  this is the handle linked to the terminal, that will terminate the
 *  program if CTRL-D
 *
 * newtork_file_actions
 *  reads a message (supposely emitted by a peer) related to files actions
 *  and execute instructions on the disk
 */
use std::env;

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};
#[cfg(target_os = "windows")]
use winfsp::winfsp_init;
use wormhole::commands::{self, cli_commands::Cli};
use wormhole::config::types::Config;
use wormhole::config::LocalConfig;
use wormhole::error::{CliError, CliSuccess, WhError, WhResult};
use wormhole::network::ip::IpP;
use wormhole::pods::pod::Pod;

type CliTcpWriter =
    SplitSink<WebSocketStream<tokio::net::TcpStream>, tokio_tungstenite::tungstenite::Message>;

async fn handle_cli_command(
    pods: &mut HashMap<String, Pod>,
    command: Cli,
    mut writer: CliTcpWriter,
) {
    let response_command = match command {
        Cli::New(pod_args) => match commands::service::new(pod_args).await {
            Ok(pod) => {
                let name = pod.get_name().to_string();
                pods.insert(name.clone(), pod);
                Ok(CliSuccess::WithData {
                    message: String::from("Pod created with success"),
                    data: name,
                })
            }
            Err(e) => Err(e),
        },
        Cli::Start(pod_args) => commands::service::start(pod_args).await,
        Cli::Stop(pod_args) => {
            if let Some(pod) = pods.remove(&pod_args.name) {
                commands::service::stop(pod).await
            } else {
                log::warn!("(TODO) Stopping a pod by path is not yet implemented");
                Err(CliError::PodRemovalFailed {
                    name: pod_args.name,
                })
            }
        }
        Cli::Remove(remove_arg) => {
            let opt = if remove_arg.name != "." {
                pods.remove(&remove_arg.name)
            } else if remove_arg.path.inner != "." {
                let key_to_remove = pods
                    .iter()
                    .find(|(_, pod)| pod.get_mount_point() == &remove_arg.path)
                    .map(|(key, _)| key.clone());

                key_to_remove.and_then(|key| pods.remove(&key))
            } else {
                log::error!("No pod name nor path were provided by RemovePod command");
                None
            };
            if let Some(pod) = opt {
                commands::service::remove(remove_arg, pod).await
            } else {
                Err(CliError::PodRemovalFailed {
                    name: remove_arg.name,
                })
            }
        }
        Cli::Restore(mut resotre_args) => {
            let opt_pod = if resotre_args.name == "." {
                pods.iter()
                    .find(|(_, pod)| pod.get_mount_point() == &resotre_args.path)
            } else {
                pods.iter().find(|(n, _)| n == &&resotre_args.name)
            };
            if let Some((_, pod)) = opt_pod {
                resotre_args.path = pod.get_mount_point().clone();
                commands::service::restore(
                    pod.local_config.clone(),
                    pod.global_config.clone(),
                    resotre_args,
                )
            } else {
                log::error!(
                    "Pod at this path doesn't existe {:?}, {:?}",
                    resotre_args.name,
                    resotre_args.path
                );
                Err(CliError::PodRemovalFailed {
                    name: resotre_args.name,
                })
            }
        }
        Cli::Apply(mut pod_conf) => {
            // Find the good pod
            let opt_pod = if pod_conf.name == "." {
                pods.iter()
                    .find(|(_, pod)| pod.get_mount_point() == &pod_conf.path)
            } else {
                pods.iter().find(|(n, _)| n == &&pod_conf.name)
            };

            //Apply new confi in the pod and check if the name change
            let res = if let Some((name, pod)) = opt_pod {
                pod_conf.path = pod.get_mount_point().clone();

                match commands::service::apply(
                    pod.local_config.clone(),
                    pod.global_config.clone(),
                    pod_conf.clone(),
                ) {
                    Err(err) => Err(err),
                    Ok(_) => {
                        match LocalConfig::read_lock(
                            &pod.local_config.clone(),
                            "handle_cli_command::apply",
                        ) {
                            Ok(local) => {
                                if local.general.name != *name {
                                    Ok(Some((local.general.name.clone(), name.clone())))
                                } else {
                                    Ok(None)
                                }
                            }
                            Err(err) => Err(CliError::WhError { source: err }),
                        }
                    }
                }
            } else {
                Err(CliError::Message {
                    reason: format!(
                        "This name or path doesn't existe in the hashmap: {:?}, {:?}",
                        pod_conf.name, pod_conf.path
                    ),
                })
            };

            // Modify the name in the hashmap if it necessary
            match res {
                Ok(Some((new_name, old_name))) => {
                    if let Some(pod) = pods.remove(&old_name) {
                        pods.insert(new_name, pod);
                        Ok(CliSuccess::Message("tt".to_owned()))
                    } else {
                        Err(CliError::Message {
                            reason: "non".to_owned(),
                        })
                    }
                }
                Ok(None) => {
                    todo!()
                }
                Err(err) => Err(err),
            }
        }
        Cli::GetHosts(args) => {
            if let Some(pod) = pods.get(&args.name) {
                match pod.get_file_hosts(args.path) {
                    Ok(hosts) => Ok(CliSuccess::WithData {
                        message: "Hosts:".to_owned(),
                        data: format!("{:?}", hosts),
                    }),
                    Err(error) => Err(CliError::PodInfoError { source: error }),
                }
            } else {
                Err(CliError::PodNotFound)
            }
        }
        Cli::Tree(args) => {
            if let Some(pod) = pods.get(&args.name) {
                match pod.get_file_tree_and_hosts(args.path) {
                    Ok(tree) => Ok(CliSuccess::WithData {
                        message: "File tree and hosts per file:".to_owned(),
                        data: tree.to_string(),
                    }),
                    Err(error) => Err(CliError::PodInfoError { source: error }),
                }
            } else {
                Err(CliError::PodNotFound)
            }
        }
        _ => Err(CliError::InvalidCommand),
    };
    let string_output = response_command.map_or_else(|e| e.to_string(), |a| a.to_string());
    match writer.send(Message::Text(string_output)).await {
        Ok(()) => log::debug!("Sent answer to cli"),
        Err(err) => log::error!("Message can't send to cli: {}", err),
    }
}

async fn get_cli_command(stream: tokio::net::TcpStream) -> WhResult<(Cli, CliTcpWriter)> {
    // Accept the TCP stream as a WebSocket stream
    let ws_stream = match accept_async(stream).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("get_cli_command: can't accept tcp stream: {}", e);
            return Err(WhError::NetworkDied {
                called_from: "get_cli_command::accept_tcp_stream".to_owned(),
            });
        }
    };
    let (writer, mut reader) = ws_stream.split();

    // Read the first message from the stream
    let message_data = match reader.next().await {
        Some(Ok(msg)) => msg.into_data(),
        Some(Err(e)) => {
            log::error!("get_cli_command: invalid message: {}", e);
            return Err(WhError::NetworkDied {
                called_from: "get_cli_command".to_owned(),
            });
        }
        None => {
            log::error!("get_cli_command: can't get message from tcp stream");
            return Err(WhError::NetworkDied {
                called_from: "get_cli_command".to_owned(),
            });
        }
    };

    // Deserialize the message data into a Cli object
    let cmd = match bincode::deserialize(&message_data) {
        Ok(c) => c,
        Err(e) => {
            log::error!("get_cli_command: failed to deserialize message: {}", e);
            return Err(WhError::NetworkDied {
                called_from: "get_cli_command::deserialize_message".to_owned(),
            });
        }
    };

    Ok((cmd, writer))
}

/// Listens for CLI calls and launch one tcp instance per cli command
async fn start_cli_listener(
    pods: &mut HashMap<String, Pod>,
    ip: String,
    mut interrupt_rx: UnboundedReceiver<()>,
) {
    let mut ip: IpP = IpP::try_from(&ip).expect("start_cli_listener: invalid ip provided");
    println!("Starting CLI's TcpListener on {}", ip.to_string());

    let mut listener = TcpListener::bind(&ip.to_string()).await;
    while let Err(e) = listener {
        log::error!(
            "Address {} not available due to {}, switching...",
            ip.to_string(),
            e
        );
        ip.set_port(ip.port + 1);
        log::debug!("Starting CLI's TcpListener on {}", ip.to_string());
        listener = TcpListener::bind(&ip.to_string()).await;
    }
    log::info!("Started CLI's TcpListener on {}", ip.to_string());
    let listener = listener.unwrap();

    while let Some(Ok((stream, _))) = tokio::select! {
        v = listener.accept() => Some(v),
        _ = interrupt_rx.recv() => None,
    } {
        let (command, writer) = match get_cli_command(stream).await {
            Ok(cmd) => cmd,
            Err(e) => {
                log::error!("cli_listener: error on getting command: {e}");
                continue;
            }
        };
        handle_cli_command(pods, command, writer).await;
    }
}

#[tokio::main]
async fn main() {
    let (interrupt_tx, interrupt_rx) = mpsc::unbounded_channel::<()>();
    let mut pods: HashMap<String, Pod> = HashMap::new();

    env_logger::init();

    #[cfg(target_os = "windows")]
    match winfsp_init() {
        Ok(_token) => println!("got fsp token!"),
        Err(err) => {
            println!("fsp error: {:?}", err);
            std::process::exit(84)
        }
    }

    let ip: String = env::args()
        .nth(1)
        .unwrap_or("127.0.0.1:8081".to_string())
        .into();
    println!("Starting service on {}", ip);

    let terminal_handle = tokio::spawn(terminal_watchdog(interrupt_tx));
    let cli_airport = start_cli_listener(&mut pods, ip, interrupt_rx);
    log::info!("Started");

    cli_airport.await;
    terminal_handle.abort();

    log::info!("Stopping");
    for (name, pod) in pods.into_iter() {
        match pod.stop()
        {
            Ok(()) => log::info!("Stopped pod {name}"),
            Err(e) => log::error!("Pod {name} can't be stopped: {e}"),
        }
    }
    log::info!("Stopped");
}

// NOTE - old watchdog brought here for debug purposes
pub async fn terminal_watchdog(tx: UnboundedSender<()>) {
    let mut stdin = tokio::io::stdin();
    let mut buf = vec![0; 1024];

    while let Ok(read) = tokio::io::AsyncReadExt::read(&mut stdin, &mut buf).await {
        // NOTE -  on ctrl-D -> quit
        match read {
            0 => {
                log::info!("Quiting!");
                let _ = tx.send(());
                return;
            }
            _ => (),
        };
    }
}
