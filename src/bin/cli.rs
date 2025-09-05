// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use clap::Parser;
use std::{env, path::PathBuf};
use wormhole::{
    commands::{self, cli_commands::Cli},
    error::CliResult,
};

fn get_config_path() -> PathBuf {
    let config_dir =
        env::var("WORMHOLE_CONFIG_DIR").unwrap_or_else(|_| ".config/wormhole".to_string());
    PathBuf::from(config_dir).join("config.toml")
}

/// Parse argument and recover the ip connection to the service or use 127.0.0.1:8081
fn get_args(args: Vec<String>) -> (String, Vec<String>) {
    let ip: String;
    let cli_args: Vec<String>;

    if let Some(first_arg) = args.get(1) {
        if first_arg.contains(':') {
            ip = first_arg.clone();
            cli_args = args.into_iter().skip(1).collect();
        } else {
            ip = "127.0.0.1:8081".to_string();
            cli_args = args;
        }
    } else {
        ip = "127.0.0.1:8081".to_string();
        cli_args = vec![];
    }
    return (ip, cli_args);
}

fn main() -> CliResult<()> {
    env_logger::init();

    // Recover all arguments
    let args: Vec<String> = env::args().collect();
    let (ip, cli_args) = get_args(args);
    let ip = ip.as_str();
    log::trace!("Starting cli on {}", ip);
    log::trace!("cli args: {:?}", cli_args);

    let status = match Cli::parse_from(cli_args) {
        Cli::Start(args) => commands::cli::start(ip, args),
        Cli::Stop(args) => commands::cli::stop(ip, args),
        Cli::Template(args) => {
            log::info!("creating network {:?}", args.name.clone());
            commands::cli::templates(&args.path, &args.name)
        }
        Cli::New(args) => {
            log::info!("creating pod");
            commands::cli::new(ip, args)
        }
        Cli::Remove(args) => {
            log::info!("removing pod");
            commands::cli::remove(ip, args)
        }
        Cli::Inspect => {
            log::warn!("inspecting pod");
            todo!("inspect");
        }
        Cli::GetHosts(args) => commands::cli::get_hosts(ip, args),
        Cli::Tree(args) => commands::cli::tree(ip, args),
        Cli::Apply(args) => {
            log::warn!("reloading pod");
            commands::cli::apply(ip, args)
        }
        Cli::Status => commands::cli::status(ip),
        Cli::Restore(args) => {
            log::info!("retore a specific file config");
            commands::cli::restore(ip, args)
        }
        Cli::Interrupt => {
            log::warn!("interrupt command");
            todo!("interrupt");
        }
    };
    if let Err(e) = &status {
        log::error!("CLI: error reported: {e}");
    } else {
        log::info!("CLI: no error reported")
    };
    status.map(|s| {
        println!("{s}");
        ()
    })
}
