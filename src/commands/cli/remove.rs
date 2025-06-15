// In rust we code
// In code we trust
// AgarthaSoftware - 2024

use tokio::runtime::Runtime;

use crate::commands::cli_commands::{Cli, RemoveArgs};
use crate::error::CliResult;
use crate::pods::whpath::WhPath;
use std::env;

use super::cli_messager;

pub fn remove(ip: &str, mut args: RemoveArgs) -> CliResult<()> {
    if args.name == "." {
        let p = env::current_dir()?;
        let path = WhPath::from(&p.display().to_string());
        args.path = if args.path.inner != "." {
            path.join(&args.path)
        } else {
            path
        }
    }
    let rt = Runtime::new().unwrap();
    rt.block_on(cli_messager(ip, Cli::Remove(args)))
}
