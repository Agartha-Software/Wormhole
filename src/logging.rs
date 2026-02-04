use std::io::Write;

use log::Level;
use owo_colors::{OwoColorize, Style};

pub fn custom_format(
    fmt: &mut env_logger::fmt::Formatter,
    record: &log::Record<'_>,
) -> std::io::Result<()> {
    let time = fmt.timestamp().to_string();
    let time = time
        .split("T")
        .nth(1)
        .and_then(|t| t.split("Z").next())
        .unwrap_or(&time);

    let level = record.level();
    let level_str = level.as_str();
    let level = match level {
        Level::Trace => level_str.style(Style::new().cyan()),
        Level::Debug => level_str.style(Style::new().blue()),
        Level::Info => level_str.style(Style::new().green()),
        Level::Warn => level_str.style(Style::new().yellow()),
        Level::Error => level_str.style(Style::new().red().bold()),
    };
    let module = record
        .module_path()
        .and_then(|m| m.rsplit("::").next())
        .unwrap_or("");

    let module = module.bright_blue();
    let left = "[".bright_black();
    let right = "]".bright_black();

    writeln!(
        fmt,
        "{left}{time} {level} {module}{right} {}",
        record.args()
    )
}
