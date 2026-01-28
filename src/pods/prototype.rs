use crate::config::local_file::LocalConfigFile;
use crate::config::GlobalConfig;
use crate::ipc::answers::InspectInfo;
use crate::network;
use crate::pods::itree::ITree;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PodPrototype {
    pub global_config: GlobalConfig,
    pub name: String,
    pub listen_addrs: Vec<Multiaddr>,
    pub mountpoint: PathBuf,
    pub should_restart: bool,
    pub allow_other_users: bool,
}

pub type ConnectionInfo = (ITree, Vec<PeerId>);

impl PodPrototype {
    pub fn apply_config(&mut self, local: LocalConfigFile) {
        if let Some(name) = local.name {
            self.name = name;
        }
        if let Some(restart) = local.restart {
            self.should_restart = restart;
        }
    }

    pub fn get_inspect_info(&self) -> InspectInfo {
        let listen_addrs = self
            .listen_addrs
            .iter()
            .map(|m| network::PeerInfo::display_address(m).unwrap_or_else(|m| m.to_string()))
            .collect();

        InspectInfo {
            frozen: true,
            listen_addrs,
            name: self.name.clone(),
            connected_peers: vec![],
            mount: self.mountpoint.clone(),
            disk_space: None,
        }
    }

    // pub async fn try_to_connect(
    //     &mut self,
    //     fail_on_network: bool,
    //     receiver_in: &UnboundedSender<FromNetworkMessage>,
    // ) -> Result<ConnectionInfo, io::Error> {
    //     if !self.global_config.general.entrypoints.is_empty() {
    //         for first_contact in &self.global_config.general.entrypoints {
    //             match PeerIPC::connect(
    //                 first_contact.to_owned(),
    //                 self.hostname.clone(),
    //                 self.public_url.clone(),
    //                 receiver_in,
    //             )
    //             .await
    //             {
    //                 Err(HandshakeError::CouldntConnect) => continue,
    //                 Err(e) => log::error!("{first_contact}: {e}"),
    //                 Ok((ipc, accept)) => {
    //                     if let Some(urls) =
    //                         accept
    //                             .urls
    //                             .into_iter()
    //                             .skip(1)
    //                             .try_fold(Vec::new(), |mut a, b| {
    //                                 a.push(b?);
    //                                 Some(a)
    //                             })
    //                     {
    //                         let new_hostname = accept.rename.unwrap_or(self.hostname.clone());

    //                         match PeerIPC::peer_startup(
    //                             urls,
    //                             new_hostname.clone(),
    //                             accept.hostname,
    //                             receiver_in,
    //                         )
    //                         .await
    //                         {
    //                             Ok(mut other_ipc) => {
    //                                 other_ipc.insert(0, ipc);

    //                                 self.hostname = new_hostname;
    //                                 self.global_config = accept.config;

    //                                 return Ok((accept.itree, other_ipc));
    //                             }

    //                             Err(e) => log::error!("a peer failed: {e}"),
    //                         };
    //                     }
    //                 }
    //             }
    //         }
    //         if fail_on_network {
    //             log::error!("None of the specified peers could answer. Stopping.");
    //             return Err(io::Error::other("None of the specified peers could answer"));
    //         }
    //     }
    //     Ok((
    //         generate_itree(&self.mountpoint, &self.hostname).unwrap_or_default(),
    //         vec![],
    //     ))
    // }
}
