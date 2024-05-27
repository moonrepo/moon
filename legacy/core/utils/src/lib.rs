pub mod fs;
pub mod path;
pub mod regex;
pub mod semver;
pub mod time;

pub use async_trait::async_trait;

use cached::proc_macro::cached;
use moon_common::consts::CONFIG_DIRNAME;
use std::env;
use std::path::PathBuf;

#[macro_export]
macro_rules! string_vec {
    () => {{
        Vec::<String>::new()
    }};
    ($($item:expr),+ $(,)?) => {{
        vec![
            $( String::from($item), )*
        ]
    }};
}

pub fn hash<T: AsRef<str>>(value: T) -> String {
    format!("{:x}", md5::compute(value.as_ref()))
}

#[cached]
pub fn get_workspace_root() -> PathBuf {
    if let Ok(root) = env::var("MOON_WORKSPACE_ROOT") {
        let root: PathBuf = root.parse().expect("Failed to parse MOON_WORKSPACE_ROOT.");

        return root;
    }

    starbase_utils::fs::find_upwards_root(
        CONFIG_DIRNAME,
        env::current_dir().expect("Invalid working directory."),
    )
    .expect("Unable to get workspace root. Is moon running?")
}

#[inline]
pub fn get_cache_dir() -> PathBuf {
    get_workspace_root().join(CONFIG_DIRNAME).join("cache")
}

#[inline]
pub fn is_ci() -> bool {
    match env::var("CI") {
        Ok(var) => !var.is_empty(),
        Err(_) => false,
    }
}

#[inline]
pub fn is_docker_container() -> bool {
    PathBuf::from("/.dockerenv").exists()
}

#[inline]
pub fn is_test_env() -> bool {
    env::var("MOON_TEST").is_ok()
}

#[inline]
pub fn is_unformatted_stdout() -> bool {
    !env::args().any(|arg| arg == "--json" || arg == "--dot")
}
