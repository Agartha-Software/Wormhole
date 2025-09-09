use crate::{
    commands::cli_commands::{Mode, RemoveArgs},
    error::{CliError, CliResult, CliSuccess},
    pods::pod::Pod,
};

async fn simple_remove(_pod: Pod) -> CliResult<CliSuccess> {
    Err(CliError::Unimplemented {
        arg: "Simple remove".into(),
    })
}

async fn clean_remove(_pod: Pod) -> CliResult<CliSuccess> {
    Err(CliError::Unimplemented {
        arg: "Clean remove".into(),
    })
}

async fn clone_remove(_pod: Pod) -> CliResult<CliSuccess> {
    Err(CliError::Unimplemented {
        arg: "Clone remove".into(),
    })
}

async fn take_remove(_pod: Pod) -> CliResult<CliSuccess> {
    Err(CliError::Unimplemented {
        arg: "Take remove".into(),
    })
}

pub async fn remove(args: RemoveArgs, pod: Pod) -> CliResult<CliSuccess> {
    match args.mode {
        Mode::Simple => simple_remove(pod).await,
        Mode::Clone => clone_remove(pod).await,
        Mode::Clean => clean_remove(pod).await,
        Mode::Take => take_remove(pod).await,
    }
}
