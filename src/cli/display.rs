use colored::{ColoredString, Colorize};
use std::io::IsTerminal;

pub fn print_err<D>(err: D)
where
    D: std::fmt::Display,
    ColoredString: From<D>,
{
    if std::io::stderr().is_terminal() {
        eprintln!("{}", ColoredString::from(err).red().bold());
    } else {
        eprintln!("{err}");
    }
}
