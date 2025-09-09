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
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use wormhole::commands::{self, cli_commands::Cli};
use wormhole::config::types::Config;
use wormhole::config::LocalConfig;
use wormhole::error::{CliError, CliSuccess, WhError, WhResult};
use wormhole::network::ip::IpP;
use wormhole::pods::pod::Pod;

#[cfg(target_os = "windows")]
use winfsp::winfsp_init;

type CliTcpWriter =
    SplitSink<WebSocketStream<tokio::net::TcpStream>, tokio_tungstenite::tungstenite::Message>;

async fn handle_cli_command(
    ip: &IpP,
    pods: &mut HashMap<String, Pod>,
    command: Cli,
    mut writer: CliTcpWriter,
) {
    let response_command = match command {
        Cli::New(pod_args) => {
            if if let Some(path) = &pod_args.mountpoint {
                pods.values().any(|p| p.get_mountpoint() == path)
            } else {
                false
            } {
                Err(CliError::Message {
                    reason: "This mount point already exist.".to_string(),
                })
            } else {
                let pod_name = pod_args.name.clone();
                match commands::service::new(pod_args).await {
                    Ok(pod) => {
                        pods.insert(pod_name.clone(), pod);
                        Ok(CliSuccess::WithData {
                            message: String::from("Pod created with success"),
                            data: pod_name,
                        })
                    }
                    Err(e) => Err(e),
                }
            }
        }
        Cli::Start(pod_args) => commands::service::start(pod_args).await,
        Cli::Stop(pod_args) => {
            let key = pod_args
                .name
                .clone()
                .ok_or(CliError::PodNotFound)
                .or_else(|_| {
                    pod_args
                        .path
                        .clone()
                        .ok_or(CliError::InvalidArgument {
                            arg: "missing both path and name args".to_owned(),
                        })
                        .and_then(|path| {
                            pods.iter()
                                .find(|(_, pod)| pod.get_mountpoint() == &path)
                                .map(|(key, _)| key.clone())
                                .ok_or(CliError::PodNotFound)
                        })
                });
            match key {
                Err(e) => Err(e),
                Ok(key) => {
                    if let Some(pod) = pods.remove(&key) {
                        commands::service::stop(pod).await
                    } else {
                        Err(CliError::PodNotFound)
                    }
                }
            }
        }
        Cli::Remove(remove_arg) => {
            let key = remove_arg
                .name
                .clone()
                .ok_or(CliError::PodNotFound)
                .or_else(|_| {
                    remove_arg
                        .path
                        .clone()
                        .ok_or(CliError::InvalidArgument {
                            arg: "missing both path and name args".to_owned(),
                        })
                        .and_then(|path| {
                            pods.iter()
                                .find(|(_, pod)| pod.get_mountpoint() == &path)
                                .map(|(key, _)| key.clone())
                                .ok_or(CliError::PodNotFound)
                        })
                });
            let pod = key.and_then(|key| pods.remove(&key).ok_or(CliError::PodNotFound));

            match pod {
                Ok(pod) => commands::service::remove(remove_arg, pod).await,
                Err(e) => Err(e),
            }
        }
        Cli::Restore(mut restore_args) => {
            let opt_pod = if let Some(name) = &restore_args.name {
                pods.iter().find(|(n, _)| n == &name)
            } else if let Some(path) = &restore_args.path {
                pods.iter().find(|(_, pod)| pod.get_mountpoint() == path)
            } else {
                None
            };
            if let Some((_, pod)) = opt_pod {
                restore_args.path = Some(pod.get_mountpoint().clone());
                commands::service::restore(
                    pod.local_config.clone(),
                    pod.global_config.clone(),
                    restore_args,
                )
            } else {
                log::error!(
                    "Pod at this path doesn't existe {:?}, {:?}",
                    restore_args.name,
                    restore_args.path
                );
                Err(CliError::PodRemovalFailed {
                    name: restore_args.name.unwrap_or("".to_owned()),
                })
            }
        }
        Cli::Apply(mut pod_conf) => {
            // Find the good pod
            let opt_pod = if let Some(name) = &pod_conf.name {
                pods.iter().find(|(n, _)| n == &name)
            } else if let Some(path) = &pod_conf.path {
                pods.iter().find(|(_, pod)| pod.get_mountpoint() == path)
            } else {
                None
            };

            //Apply new confi in the pod and check if the name change
            let res = if let Some((name, pod)) = opt_pod {
                pod_conf.path = Some(pod.get_mountpoint().clone());

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
                                Ok(None)
                                // if local.general.name != *name {
                                //     Ok(Some((local.general.name.clone(), name.clone())))
                                // } else {
                                //     Ok(None)
                                // }
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
                    let old_name: String = old_name;
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
            if let Some((_, pod)) = if let Some(name) = &args.name {
                pods.iter().find(|(n, _)| n == &name)
            } else if let Some(path) = &args.path {
                pods.iter().find(|(_, pod)| pod.get_mountpoint() == path)
            } else {
                None
            } {
                match pod.get_file_hosts(args.path.unwrap_or(".".into())) {
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
            let path = args.path.and_then(|path| std::fs::canonicalize(&path).ok());
            log::info!("TREE: canonical: {path:?}");
            if let Some((pod, subpath)) = {
                if let Some(name) = &args.name {
                    pods.iter()
                        .find_map(|(n, pod)| (n == name).then_some((pod, None)))
                } else if let Some(path) = &path {
                    pods.iter().find_map(|(_, pod)| {
                        log::info!("TREE: pod: {:?}", &pod.get_mountpoint());
                        path.strip_prefix(&pod.get_mountpoint())
                            .ok()
                            .map(|sub| (pod, Some(sub.into())))
                    })
                } else {
                    None
                }
            } {
                match pod.get_file_tree_and_hosts(subpath) {
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
        Cli::Template(_template_arg) => todo!(),
        Cli::Inspect => todo!(),
        Cli::Status => Ok(CliSuccess::Message(format!("{}", ip.to_string()))),
        Cli::Interrupt => todo!(),
    };
    let string_output = response_command
        .inspect_err(|e| log::error!("handling cli: {e:?}"))
        .map_or_else(|e| format!("CliError: {:?}", e), |a| a.to_string());
    match writer.send(Message::Text(string_output.into())).await {
        Ok(()) => log::debug!("Sent answer to cli"),
        Err(err) => log::error!("Message can't send to cli: {}", err),
    }
}

async fn get_cli_command(stream: tokio::net::TcpStream) -> WhResult<(Cli, CliTcpWriter)> {
    // Accept the TCP stream as a WebSocket stream
    let ws_stream = match tokio_tungstenite::accept_async_with_config(
        stream,
        Some(
            WebSocketConfig::default()
                .max_message_size(None)
                .max_frame_size(None),
        ),
    )
    .await
    {
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

const MAX_TRY_PORTS: u16 = 10;
const MAX_PORT: u16 = 65535;

custom_error::custom_error! {CliListenerError
    ProvidedIpNotAvailable {ip: String, err: String} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMainPort {max_port: u16} = "Unable to start cli_listener (not testing ports above {max_port})",
    AboveMaxTry {max_try_port: u16} = "Unable to start cli_listener (tested {max_try_port} ports)",
}

/// Listens for CLI calls and launch one tcp instance per cli command
/// if `specific_ip` is not given, will try all ports starting from 8081 to 9999, incrementing until success
/// if `specific_ip` is given, will try the given ip and fail on error.
async fn start_cli_listener(
    pods: &mut HashMap<String, Pod>,
    specific_ip: Option<String>,
    mut signals_rx: UnboundedReceiver<()>,
) -> Result<(), CliListenerError> {
    let mut ip: IpP = IpP::try_from(&specific_ip.clone().unwrap_or(DEFAULT_ADDRESS.to_string()))
        .expect("start_cli_listener: invalid ip provided");
    println!("Starting CLI's Listener on {}", ip.to_string());

    let mut port_tries_count = 0;
    let mut listener = TcpListener::bind(&ip.to_string()).await;
    while let Err(e) = listener {
        if let Some(_) = specific_ip {
            return Err(CliListenerError::ProvidedIpNotAvailable {
                ip: ip.to_string(),
                err: e.to_string(),
            });
        }
        if ip.port >= MAX_PORT {
            return Err(CliListenerError::AboveMainPort { max_port: MAX_PORT });
        }
        if port_tries_count > MAX_TRY_PORTS {
            return Err(CliListenerError::AboveMaxTry {
                max_try_port: MAX_TRY_PORTS,
            });
        }
        log::warn!(
            "Address {} not available due to {}, switching...",
            ip.to_string(),
            e
        );
        ip.set_port(ip.port + 1);
        port_tries_count += 1;
        log::debug!("Starting CLI's TcpListener on {}", ip.to_string());
        listener = TcpListener::bind(&ip.to_string()).await;
    }
    log::info!("Started CLI's TcpListener on {}", ip.to_string());
    let listener = listener.unwrap();

    while let Some(Ok((stream, _))) = tokio::select! {
        v = listener.accept() => Some(v),
        _ = signals_rx.recv() => None,
    } {
        let (command, writer) = match get_cli_command(stream).await {
            Ok(cmd) => cmd,
            Err(e) => {
                log::error!("cli_listener: error on getting command: {e}");
                continue;
            }
        };
        handle_cli_command(&ip, pods, command, writer).await;
    }
    Ok(())
}

const DEFAULT_ADDRESS: &str = "127.0.0.1:8081";

#[tokio::main]
async fn main() {
    let (interrupt_tx, interrupt_rx) = mpsc::unbounded_channel::<()>();
    let (signals_tx, signals_rx) = mpsc::unbounded_channel::<()>();

    let mut pods: HashMap<String, Pod> = HashMap::new();

    if env::args().any(|arg| arg == "-h" || arg == "--help") {
        println!("Usage: wormholed <IP>\n\nIP is the node address, default at {DEFAULT_ADDRESS}");
        return;
    }

    env_logger::init();

    #[cfg(target_os = "windows")]
    match winfsp_init() {
        Ok(_token) => log::debug!("got fsp token!"),
        Err(err) => {
            log::error!("fsp error: {:?}", err);
            std::process::exit(84)
        }
    }

    let ip_string = env::args().nth(1);
    let terminal_handle = tokio::spawn(terminal_watchdog(interrupt_tx));
    let signals_task = tokio::spawn(handle_signals(signals_tx, interrupt_rx));
    log::trace!("Starting service on {:?}", ip_string);
    log::info!("Started");
    let _ = start_cli_listener(&mut pods, ip_string, signals_rx).await;

    terminal_handle.abort();
    signals_task.await.unwrap();

    log::info!("Stopping");
    for (name, pod) in pods.into_iter() {
        match pod.stop().await {
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
                let _ = tx.send(());
                return;
            }
            _ => (),
        };
    }
}

async fn handle_signals(tx: UnboundedSender<()>, interrupt_rx: UnboundedReceiver<()>) {
    #[cfg(unix)]
    {
        handle_signals_unix(tx, interrupt_rx).await;
    }

    #[cfg(windows)]
    {
        handle_signals_windows(tx, interrupt_rx).await;
    }
}

#[cfg(unix)]
async fn handle_signals_unix(tx: UnboundedSender<()>, mut interrupt_rx: UnboundedReceiver<()>) {
    use tokio::signal::unix;

    let mut sigint = unix::signal(unix::SignalKind::interrupt()).expect("failed to bind SIGINT");
    let mut sigterm = unix::signal(unix::SignalKind::terminate()).expect("failed to bind SIGTERM");

    log::info!("Unix signal handler initialised, waiting for SIGINT or SIGTERM…");

    tokio::select! {
        _ = sigint.recv() => {
            log::info!("Quiting by Signal: SIGINT");
            let _ = tx.send(());
        }
        _ = sigterm.recv() => {
            log::info!("Quiting by Signal: SIGTERM");
            let _ = tx.send(());
        }
        _ = interrupt_rx.recv() => {
            log::info!("Quiting by Ctrl+D! (EOF)");
            let _ = tx.send(());
        }
    }
}

#[cfg(windows)]
async fn handle_signals_windows(tx: UnboundedSender<()>, mut interrupt_rx: UnboundedReceiver<()>) {
    log::info!("Windows signal handler initialised…");

    let mut sig_c = tokio::signal::windows::ctrl_c().expect("Failed to register ctrl_c");
    let mut sig_break =
        tokio::signal::windows::ctrl_break().expect("Failed to register ctrl_break");

    tokio::select! {
        _ = sig_c.recv() => {
            log::info!("Quiting by Signal: CTRL+C");
            let _ = tx.send(());
        }
        _ = sig_break.recv() => {
            log::info!("Quiting by Signal: CTRL+BREAK");
            let _ = tx.send(());
        }
        _ = interrupt_rx.recv() => {
            log::info!("Quiting by Ctrl-Z (EOF)");
            let _ = tx.send(());
        }
    }
}
