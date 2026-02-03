use std::io::Write;

pub fn custom_format(fmt: &mut env_logger::fmt::Formatter, record: &log::Record<'_>) -> std::io::Result<()> {
    let time = fmt.timestamp().to_string();
    let time = time.split("T").nth(1).and_then(|t|t.split("Z").next()).unwrap_or(&time);

    let subtle_style = anstyle::AnsiColor::BrightBlack.on_default();

    let level = record.level();
    let level_style = fmt.default_level_style(level);

    let module = record.module_path().and_then(|m|m.rsplit("::").next()).unwrap_or("");
    let module_style = anstyle::AnsiColor::BrightBlue.on_default();

    write!(fmt, "{subtle_style}[{subtle_style:#}")?;
    write!(fmt, "{time}")?;
    write!(fmt, " {level_style}{level}{level_style:#}")?;
    write!(fmt, " {module_style}{module}{module_style:#}")?;
    write!(fmt, "{subtle_style}] {subtle_style:#}")?;

    writeln!(fmt, "{}", record.args())
}