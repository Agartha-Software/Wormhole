use crate::{
    commands::cli_commands::StatusPodArgs,
    error::{CliResult, CliSuccess},
};

pub async fn start(start_args: StatusPodArgs) -> CliResult<CliSuccess> {
    let name = start_args.name.clone();
    Ok(CliSuccess::WithData { message: String::from("Pod start: "), data: name })
}
