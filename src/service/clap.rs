use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[command(about, long_about = None)]
pub struct ServiceArgs {
    #[arg(long)]
    pub nodeamon: bool,
    #[arg(short)]
    pub ip: Option<String>,
    #[arg(short)]
    pub socket: Option<String>,
    #[arg(short, long)]
    pub clean: bool,
    #[arg(long)]
    pub allow_other_users: bool,
}
