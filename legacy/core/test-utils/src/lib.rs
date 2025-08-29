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

use moon_common::path::{WorkspaceRelativePathBuf, clean_components};
use moon_config::Input;
use std::path::PathBuf;
use std::str::FromStr;

pub fn get_fixtures_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.push("../../../tests/fixtures");
    clean_components(root)
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

pub fn create_inputs<I, V>(list: I) -> Vec<Input>
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    list.into_iter()
        .map(|path| Input::from_str(path.as_ref()).unwrap())
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
