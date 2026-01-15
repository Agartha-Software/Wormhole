use std::{collections::HashMap, io};

use interprocess::local_socket::tokio::Stream;

use crate::{
    cli::IdentifyPodArgs,
    ipc::{
        answers::RedundancyStatusAnswer,
        cli::connection::{recieve_answer, send_command},
        commands::{Command, PodId},
    },
    pods::network::redundancy::RedundancyStatus,
};

fn display_status(status: HashMap<RedundancyStatus, u64>) -> String {
    status
        .iter()
        .map(|(s, nb)| match s {
            RedundancyStatus::AboveTarget => format!("\tFiles above target:\t\t{nb}"),
            RedundancyStatus::BelowTarget => format!("\tFiles below target:\t\t{nb}"),
            RedundancyStatus::NotRedundant => format!("\tFiles without any redundancies:\t{nb}"),
            RedundancyStatus::OnTarget => format!("\tFiles on target:\t\t{nb}"),
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub async fn redundancy_status(args: IdentifyPodArgs, mut stream: Stream) -> io::Result<String> {
    let pod = PodId::from(args);

    send_command(Command::RedundancyStatus(pod), &mut stream).await?;

    match recieve_answer::<RedundancyStatusAnswer>(&mut stream).await? {
        RedundancyStatusAnswer::InternalError => Ok("Internal server error.".to_owned()),
        RedundancyStatusAnswer::PodNotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The given pod couldn't be found.",
        )),
        RedundancyStatusAnswer::Status(status) => Ok(display_status(status)),
    }
}
