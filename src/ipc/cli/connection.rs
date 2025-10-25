use std::io;

use crate::ipc::commands::Command;
use interprocess::local_socket::{
    tokio::{prelude::*, Stream},
    GenericFilePath, GenericNamespaced,
};
use serde::Deserialize;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start_local_socket() -> io::Result<Stream> {
    let name = if GenericNamespaced::is_supported() {
        "wormhole.sock".to_ns_name::<GenericNamespaced>()?
    } else {
        "/tmp/wormhole.sock".to_fs_name::<GenericFilePath>()?
    };
    Stream::connect(name).await
}

pub async fn send_command(command: Command, stream: &mut Stream) -> Result<(), std::io::Error> {
    let serialized =
        bincode::serialize(&command).expect("Can't serialize cli command, shouldn't be possible .");

    stream.write_all(&serialized).await
}

pub async fn recieve_answer<T>(stream: &mut Stream) -> Result<T, std::io::Error>
where
    T: for<'a> Deserialize<'a>,
{
    let mut recived_answer = Vec::new();

    stream.read_buf(&mut recived_answer).await?;
    bincode::deserialize::<T>(&recived_answer).map_err(|err| {
        log::error!("Service answer deserialisation failed: {err}");
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Command recieved isn't recognized",
        )
    })
}
