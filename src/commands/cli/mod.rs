mod apply;
mod get_hosts;
mod message;
mod new;
mod register;
mod remove;
mod restore;
mod start;
mod status;
mod stop;
mod templates;
mod tree;

use std::{env, path::Path};

pub use apply::apply;
pub use get_hosts::get_hosts;
pub use message::cli_messager;
pub use new::new;
pub use register::register;
pub use remove::remove;
pub use restore::restore;
pub use start::start;
pub use status::status;
pub use stop::stop;
pub use templates::templates;
pub use tree::tree;

use crate::{
    error::{CliError, CliResult},
    pods::whpath::WhPath,
};

fn path_or_wd<'a, P: AsRef<Path> + From<&'a str>>(path: Option<P>) -> CliResult<WhPath> {
    Ok(match path {
        Some(path) => Ok(env::current_dir()?.join(path)),
        None => env::current_dir(),
    }?
    .to_str()
    .ok_or(CliError::InvalidArgument {
        arg: "path".to_owned(),
    })?
    .into())
}
