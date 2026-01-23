use interprocess::local_socket::tokio::Stream;
use std::io;

use crate::ipc::answers::{InspectAnswer, PeerInfo};

use crate::{
    cli::connection::{recieve_answer, send_command},
    cli::IdentifyPodArgs,
    ipc::commands::{Command, PodId},
};

fn display_peers(peers: Vec<PeerInfo>) -> String {
    if peers.is_empty() {
        "[ ]".to_string()
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
            "Pod informations: {}\n\
            \x20  Name:\t\t{}\n\
            \x20  Mount:\t\t{:#?}\n\
            \x20  Listen Address:\t\t{}\n\
            \x20  Connected peers:\t{}",
            if info.frozen { "Frozen" } else { "Running" },
            info.name,
            info.mount,
            info.listen_address,
            if info.frozen {
                "Disconnected (Frozen)".to_string()
            } else {
                display_peers(info.connected_peers)
            }
        )),
        InspectAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
    }
}
