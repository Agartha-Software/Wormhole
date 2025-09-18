use std::collections::HashMap;

use tokio::net::TcpListener;

use crate::network::{ip::IpP, peer_ipc::PeerIPC, server::Server};

const DEFAULT_CLI_ADDRESS: &str = "0.0.0.0:8080";
const MAX_TRY_PORTS: u16 = 15;
const MAX_PORT: u16 = 65535;

custom_error::custom_error! {CliRunnerError
    ProvidedIpNotAvailable {ip: IpP, err: std::io::Error} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMaxPort = "Unable to start cli_listener (excedeed max port)",
    AboveMaxTry = "Unable to start cli_listener (exedeed the number of tries)",
}

struct CliState {
    listener: TcpListener,
    connected_endpoints: HashMap<usize, PeerIPC>,
}

/// Create the tcp listener for the cli
///
/// If `ip` is provided, will only try using it.
/// If `ip` is not provided, will use `DEFAULT_CLI_ADDRESS` and increment the port up to `MAX_TRY_PORTS` until success.
async fn create_listener(ip: Option<IpP>) -> Result<(TcpListener, IpP), CliRunnerError> {
    if let Some(ip) = ip {
        Ok((
            TcpListener::bind(&ip.to_string()).await.map_err(|e| {
                CliRunnerError::ProvidedIpNotAvailable {
                    ip: ip.clone(),
                    err: e,
                }
            })?,
            ip,
        ))
    } else {
        let mut ip: IpP = IpP::try_from(DEFAULT_CLI_ADDRESS).unwrap();
        let mut current_try = 0;
        let mut listener = TcpListener::bind(&ip.to_string()).await;

        while let Err(e) = listener {
            log::warn!("Cli listener can't use address {ip} because of {e}");
            ip.set_port(ip.port + 1);
            current_try += 1;

            if current_try > MAX_TRY_PORTS {
                return Err(CliRunnerError::AboveMaxTry);
            };
            if ip.port > MAX_PORT {
                return Err(CliRunnerError::AboveMaxPort);
            }
            listener = TcpListener::bind(&ip.to_string()).await;
        }
        Ok((listener.unwrap(), ip))
    }
}

async fn cli(ip: Option<IpP>) -> Result<(), CliRunnerError> {
    let (listener, ip) = create_listener(ip).await?;
    Ok(())
}
