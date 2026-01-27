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
    /// nickname name to report to the network
    /// defaults to your machine's hostname
    #[arg(long, short='n')]
    pub nickname: Option<String>,
}
