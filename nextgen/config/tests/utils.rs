use schematic::{Config, ConfigError};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;

pub fn test_load_config<T, F>(file: &str, code: &str, callback: F) -> T
where
    T: Config,
    F: FnOnce(&Path) -> Result<T, ConfigError>,
{
    let sandbox = create_empty_sandbox();

    sandbox.create_file(file, code);

    match callback(sandbox.path()) {
        Ok(config) => config,
        Err(error) => panic!("{}", error.to_full_string()),
    }
}
