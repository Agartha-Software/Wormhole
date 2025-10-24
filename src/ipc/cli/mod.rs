use std::io;

use interprocess::local_socket::{
    tokio::{prelude::*, Stream},
    GenericFilePath, GenericNamespaced,
};

pub async fn start_local_socket() -> io::Result<interprocess::local_socket::tokio::Stream> {
    let name = if GenericNamespaced::is_supported() {
        "wormhole.sock".to_ns_name::<GenericNamespaced>()?
    } else {
        "/tmp/wormhole.sock".to_fs_name::<GenericFilePath>()?
    };
    Stream::connect(name).await
}
