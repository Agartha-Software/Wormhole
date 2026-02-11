use std::io::IsTerminal;

use owo_colors::OwoColorize;

pub fn print_err<D>(err: D)
where
    D: std::fmt::Display,
{
    if std::io::stderr().is_terminal() {
        eprintln!("{}", err.red().bold());
    } else {
        eprintln!("{err}");
    }
}
