use std::io;

use crate::ipc::answers::{InspectAnswer, PeerInfo};
use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
};

fn display_peers(peers: Vec<PeerInfo>) -> String {
    if peers.len() == 0 {
        format!("[ ]")
    } else {
        let mut string = String::from("[");
        for (idx, peer) in peers.iter().enumerate() {
            string.push_str("\n      { ");
            string.push_str(&peer.to_string());
            string.push_str(" }");
            if idx + 1 != peers.len() {
                string.push(',');
            }
        }
        string.push_str("\n   ]");
        string
    }
}

pub async fn inspect(args: IdentifyPodArgs, mut stream: Stream) -> Result<String, io::Error> {
    let id = PodId::from(args);

    send_command(Command::Inspect(id), &mut stream).await?;
    match recieve_answer::<InspectAnswer>(&mut stream).await? {
        InspectAnswer::Information(info) => Ok(format!(
            "Pod informations:\n\
            \x20  Name:\t\t{}\n\
            \x20  Mount:\t\t{:#?}\n\
            \x20  Hostname:\t\t{}\n\
            \x20  Url:\t\t\t{}\n\
            \x20  Bound Address:\t{}\n\
            \x20  Connected peers:\t{}\n\
            \x20  Free space:\t{}\n\
            \x20  Used space:\t{}\n\
            \x20  Total space:\t{}",
            info.name,
            info.mount,
            info.hostname,
            info.public_url.unwrap_or("[ None ]".to_string()),
            info.bound_socket,
            display_peers(info.connected_peers),
            info.disk_space
                .as_ref()
                .map_or("Error".to_owned(), |s| s.free_size.to_string()),
            info.disk_space
                .as_ref()
                .map_or("Error".to_owned(), |s| (s.total_size - s.free_size).to_string()),
            info.disk_space
                .as_ref()
                .map_or("Error".to_owned(), |s| s.total_size.to_string()),
        )),
        InspectAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
    }
}
