use crate::commands::cli_commands::IdentifyPodArgs;

pub fn start(mut start_args: IdentifyPodArgs) -> Result<String, ()> {
    // if start_args.name.is_none() {
    //     start_args.path = Some(path_or_wd(start_args.path)?)
    // }

    // let rt = Runtime::new().unwrap();
    // rt.block_on(cli_messager(ip, Cli::Start(start_args)))
    return Err(());
}
