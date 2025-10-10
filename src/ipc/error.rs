custom_error::custom_error! {pub CliListenerError
    ProvidedIpNotAvailable {ip: String, err: String} = "The specified address ({ip}) not available ({err})\nThe service is not starting.",
    AboveMainPort {max_port: u16} = "Unable to start cli_listener (not testing ports above {max_port})",
    AboveMaxTry {max_try_port: u16} = "Unable to start cli_listener (tested {max_try_port} ports)",
}
