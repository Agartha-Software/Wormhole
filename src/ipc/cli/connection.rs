use std::io;

use crate::ipc::commands::Command;
use interprocess::local_socket::{
    tokio::{prelude::*, Stream},
    GenericFilePath, GenericNamespaced,
};
use serde::Deserialize;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start_local_socket(socket: &str) -> io::Result<Stream> {
    let name = if GenericNamespaced::is_supported() {
        socket.to_ns_name::<GenericNamespaced>()?
    } else {
        format!("/tmp/{socket}").to_fs_name::<GenericFilePath>()?
    };
    Stream::connect(name).await
}

pub async fn send_command(command: Command, stream: &mut Stream) -> Result<(), std::io::Error> {
    let serialized =
        bincode::serialize(&command).expect("Can't serialize cli command, shouldn't be possible .");
    log::trace!("Sending cmd: {:?}, bytes {}", command, serialized.len());

    stream.write_u32(serialized.len() as u32).await?;
    stream.write_all(&serialized).await
}

pub async fn recieve_answer<T>(stream: &mut Stream) -> Result<T, std::io::Error>
where
    T: for<'a> Deserialize<'a> + std::fmt::Debug,
{
    let size = stream.read_u32().await?;
    let mut recived_answer = Vec::with_capacity(size as usize);

    stream.read_buf(&mut recived_answer).await?;

    bincode::deserialize::<T>(&recived_answer)
        .map_err(|err| {
            log::error!("Service answer deserialisation failed: {err}");
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Command recieved isn't recognized",
            )
        })
        .inspect(|answer| log::trace!("Recieved answer: {:?}", answer))
}
