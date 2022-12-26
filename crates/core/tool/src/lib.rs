mod errors;
mod manager;
mod tool;

pub use errors::*;
pub use manager::*;
pub use tool::*;

use std::env;
use std::path::Path;

/// We need to ensure that our toolchain binaries are executed instead of
/// other binaries of the same name. Otherwise, tooling like nvm will
/// intercept execution and break our processes. We can work around this
/// by prepending the `PATH` environment variable.
pub fn get_path_env_var(bin_dir: &Path) -> std::ffi::OsString {
    let path = env::var("PATH").unwrap_or_default();
    let mut paths = vec![bin_dir.to_path_buf()];

    paths.extend(env::split_paths(&path).collect::<Vec<_>>());

    env::join_paths(paths).unwrap()
}
