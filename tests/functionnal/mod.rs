pub mod environnement_manager;
pub mod test_sync;
pub mod test_transfer;

use std::path::PathBuf;

pub use environnement_manager::EnvironnementManager;

fn append_to_path(p: &PathBuf, s: &str) -> PathBuf {
    let mut p = p.as_os_str().to_owned();
    p.push(s);
    p.into()
}
