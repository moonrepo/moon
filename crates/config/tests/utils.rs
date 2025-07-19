#![allow(dead_code)]

use moon_config::{FileInput, GlobInput, Uri};
use schematic::{Config, ConfigError};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;

pub fn unwrap_config_result<T>(result: miette::Result<T>) -> T {
    match result {
        Ok(config) => config,
        Err(error) => {
            panic!(
                "{}",
                error.downcast::<ConfigError>().unwrap().to_full_string()
            )
        }
    }
}

pub fn test_config<P, T, F>(path: P, callback: F) -> T
where
    P: AsRef<Path>,
    T: Config,
    F: FnOnce(&Path) -> miette::Result<T>,
{
    unwrap_config_result(callback(path.as_ref()))
}

pub fn test_load_config<T, F>(file: &str, code: &str, callback: F) -> T
where
    T: Config,
    F: FnOnce(&Path) -> miette::Result<T>,
{
    let sandbox = create_empty_sandbox();

    sandbox.create_file(file, code);

    unwrap_config_result(callback(sandbox.path()))
}

pub fn test_parse_config<T, F>(code: &str, callback: F) -> T
where
    T: Config,
    F: FnOnce(&str) -> miette::Result<T>,
{
    unwrap_config_result(callback(code))
}

pub fn create_file_input(path: &str) -> FileInput {
    FileInput::from_uri(
        Uri::parse(if path.starts_with("file://") {
            path.to_owned()
        } else {
            format!("file://{path}")
        })
        .unwrap(),
    )
    .unwrap()
}

pub fn create_glob_input(path: &str) -> GlobInput {
    GlobInput::from_uri(
        Uri::parse(if path.starts_with("glob://") {
            path.to_owned()
        } else {
            format!("glob://{path}")
        })
        .unwrap(),
    )
    .unwrap()
}
