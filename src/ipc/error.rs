use custom_error::custom_error;

custom_error! {pub ListenerError
    TCPListenerError { source: TCPListenerError } = "{source}",
    SocketListenerError { source: SocketListenerError } = "{source}",
    ConnectionError { source: ConnectionError } = "{source}",
}

custom_error! {pub TCPListenerError
    ProvidedIpNotAvailable {ip: String, err: std::io::Error} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMainPort {max_port: u16} = "Unable to start the TCP listener (not testing ports above {max_port})",
    AboveMaxTry {max_try_port: u16} = "Unable to start TCP listener (tested {max_try_port} ports)",
}

custom_error! {pub SocketListenerError
    AddrInUse { name: &'static str } = "Could not start the server because the socket file is occupied. Please check\nif {name} is in use by another process and try again."
}

custom_error! {pub ConnectionError
    ImpossibleCommandRecived = "Command recieved isn't recognized",
}
