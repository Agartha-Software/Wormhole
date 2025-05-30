use std::env;

use tokio::runtime::Runtime;

use crate::{commands::cli_commands::{Cli, PodConf}, error::{CliError, CliResult}, pods::{arbo::{GLOBAL_CONFIG_FNAME, LOCAL_CONFIG_FNAME}, whpath::WhPath}};

use super::cli_messager;


pub fn apply(ip: &str, mut args: PodConf) -> CliResult<()> {
  let files_name = vec![LOCAL_CONFIG_FNAME, GLOBAL_CONFIG_FNAME];
    
    for file in args.files.clone() {
      if !files_name.contains(&file.as_str()) {
        return Err(CliError::FileConfigName { name: file });
      }
    }
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
    rt.block_on(cli_messager(
      ip,
      Cli::Apply(PodConf { name: args.name, path: args.path, files: args.files }),
    ))?;
    Ok(())
}