use crate::{ipc::error::TCPListenerError, network::ip::IpP};
use tokio::net::TcpListener;

const MAX_TRY_PORTS: u16 = 10;
const MAX_PORT: u16 = 65535;
const DEFAULT_ADDRESS: &str = "127.0.0.1:8081";

pub async fn new_tcp_listener(
    specific_ip: Option<String>,
) -> Result<TcpListener, TCPListenerError> {
    let (tcp_listener, ip) = match specific_ip {
        Some(ip) => (
            TcpListener::bind(&ip).await.map_err(|err| {
                TCPListenerError::ProvidedIpNotAvailable {
                    ip: ip.to_string(),
                    err,
                }
            })?,
            ip,
        ),
        None => free_tcp_listener().await?,
    };
    println!("Started Tcp Listener at '{}'", ip.to_string());
    Ok(tcp_listener)
}

async fn free_tcp_listener() -> Result<(TcpListener, String), TCPListenerError> {
    let mut ip: IpP = IpP::try_from(DEFAULT_ADDRESS).expect("Invalid ip provided");

    let mut port_tries_count = 0;
    loop {
        match TcpListener::bind(&ip.to_string()).await {
            Ok(listener) => break Ok((listener, ip.to_string())),
            Err(err) => {
                if ip.port >= MAX_PORT {
                    break Err(TCPListenerError::AboveMainPort { max_port: MAX_PORT });
                }
                if port_tries_count > MAX_TRY_PORTS {
                    break Err(TCPListenerError::AboveMaxTry {
                        max_try_port: MAX_TRY_PORTS,
                    });
                }
                log::warn!("Address {ip} not available due to {err}, switching...",);
                ip.set_port(ip.port + 1);
                port_tries_count += 1;
            }
        }
    }
}
