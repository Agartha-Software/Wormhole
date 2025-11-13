pub mod parser;
pub mod types;

pub use parser::parse_toml_file;
pub use types::GlobalConfig;
pub use types::LocalConfig;
