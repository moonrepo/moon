use std::env;
use std::path::PathBuf;

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
