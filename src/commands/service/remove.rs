use crate::{
    commands::cli_commands::{Mode, RemoveArgs},
    error::{CliError, CliResult, CliSuccess}, pods::pod::Pod,
};

pub async fn remove(args: RemoveArgs, _pod: Pod) -> CliResult<CliSuccess> {
    match args.mode {
        Mode::Simple => Err(CliError::Unimplemented{ arg: "Mode Simple".into() }),
        Mode::Clone => Err(CliError::Unimplemented { arg: "Mode Clone".into() }),
        Mode::Clean => Err(CliError::Unimplemented{ arg: "Mode Clean".into() }),
        Mode::Take => Err(CliError::Unimplemented{ arg: "Mode Take".into() }),
    }
}
