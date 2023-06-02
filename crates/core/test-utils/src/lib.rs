mod cli;
mod configs;
mod sandbox;

pub use assert_cmd;
pub use assert_fs;
pub use cli::*;
pub use configs::*;
pub use insta::*;
pub use predicates;
pub use pretty_assertions;
pub use sandbox::*;

use clean_path::Clean;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::InputPath;
use std::path::PathBuf;
use std::str::FromStr;

pub fn get_fixtures_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.push("../../../tests/fixtures");
    root.clean()
}

pub fn get_fixtures_path<T: AsRef<str>>(name: T) -> PathBuf {
    let path = get_fixtures_root().join(name.as_ref());

    if !path.exists() {
        panic!(
            "{}",
            format!("Fixture {} does no exist.", path.to_string_lossy())
        );
    }

    path
}

pub fn create_input_paths<I, V>(list: I) -> Vec<InputPath>
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    list.into_iter()
        .map(|path| InputPath::from_str(path.as_ref()).unwrap())
        .collect()
}

pub fn create_workspace_paths_with_prefix<I, V>(
    prefix: &str,
    list: I,
) -> Vec<WorkspaceRelativePathBuf>
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    list.into_iter()
        .map(|path| WorkspaceRelativePathBuf::from(format!("{}/{}", prefix, path.as_ref())))
        .collect()
}
