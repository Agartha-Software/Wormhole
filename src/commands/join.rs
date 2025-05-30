// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use crate::{
    commands,
    config::{self, types::Config},
};
use std::error::Error;

#[must_use]
pub fn join(
    path: &std::path::PathBuf,
    url: String,
    mut additional_hosts: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let split = url.split(':');
    let slice = &(split.collect::<Vec<_>>())[..];
    if let [address_str, network_name_str] = *slice {
        println!("passed: {:?}", slice);
        let mut peers = vec![address_str.to_owned()];
        peers.append(&mut additional_hosts);
        let network = config::Network::new(peers, network_name_str.to_owned());
        commands::templates(path, network_name_str)?;
        network.write((&path).join(".wormhole/network.toml"))?;
        return Ok(());
    } else {
        println!("errored: {:?}", slice);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "url invalid",
        )));
    }
}
