pub mod local_file;
pub mod parser;
pub mod types;

pub use parser::parse_toml_file;
pub use types::GlobalConfig;
