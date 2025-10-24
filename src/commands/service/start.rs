use serde::{Deserialize, Serialize};
use tokio::io::WriteHalf;

use crate::ipc::error::CommandResult;
use crate::pods::pod::Pod;
use crate::{commands::cli_commands::IdentifyPodArgs, error::CliSuccess};
use std::collections::HashMap;
use tokio_stream::Stream;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum StartAnswer {}

pub async fn start<Stream>(
    start_args: IdentifyPodArgs,
    pods: &mut HashMap<String, Pod>,
    write: WriteHalf<Stream>,
) -> CommandResult<()>
where
    Stream: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    let name = start_args.name.clone();
    // Ok(CliSuccess::WithData {
    //     message: String::from("Pod start: "),
    //     data: name.unwrap_or("".to_owned()),
    // })
    Ok(())
}
