// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use clap::Parser;
use std::env;
use wormhole::commands::{
        self,
        cli_commands::Cli,
    };

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Récupérer tous les arguments
    let args: Vec<String> = env::args().collect();
    let (ip, cli_args) = get_args(args);
    let ip = ip.as_str();
    println!("Starting cli on {}", ip);
    println!("cli args: {:?}", cli_args);

    match Cli::parse_from(cli_args) {
        Cli::Start(args) => commands::cli::start(ip, args)?,
        Cli::Stop(args) => commands::cli::stop(ip, args)?,
        Cli::Template(args) => {
            println!(
                "creating network {:?}",
                args.name.clone()
            );
            commands::cli::templates(
                &args.path,
                &args.name,
            )?;
        }
        Cli::New(args) => {
            println!("creating pod");
            commands::cli::new(ip, args)?;
        }
        Cli::Remove(args) => {
            println!("removing pod");
            commands::cli::remove(ip, args)?;
        }
        Cli::Inspect => {
            log::warn!("inspecting pod");
            todo!("inspect");
        }
        Cli::Reload(_args) => {
            log::warn!("reloading pod");
            todo!("reload");
        }
        Cli::Restore(args) => {
            println!("retore a specifique file config");
            commands::cli::restore(ip, args)?;
        }
        Cli::Interrupt => {
            log::warn!("do interrupt command");
            todo!("interrupt");
        }
    }
    Ok(())
}
