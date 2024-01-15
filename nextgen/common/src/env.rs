use crate::consts::CONFIG_DIRNAME;
use std::env;
use std::path::PathBuf;

#[inline]
pub fn get_moon_dir() -> PathBuf {
    if let Ok(root) = env::var("MOON_HOME") {
        return root.into();
    }

    dirs::home_dir()
        .expect("Invalid home directory.")
        .join(CONFIG_DIRNAME)
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
    env::var("MOON_TEST").is_ok() || env::var("STARBASE_TEST").is_ok()
}

#[inline]
pub fn is_unformatted_stdout() -> bool {
    !env::args().any(|arg| arg == "--json" || arg == "--dot")
}
