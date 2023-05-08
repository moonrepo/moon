mod id;

use std::env;

pub use id::*;

// Error handling
pub use miette::Diagnostic;
pub use starbase_styles::*;
pub use thiserror::Error;

#[inline]
pub fn is_ci() -> bool {
    match env::var("CI") {
        Ok(var) => !var.is_empty(),
        Err(_) => false,
    }
}

#[inline]
pub fn is_docker_container() -> bool {
    std::path::PathBuf::from("/.dockerenv").exists()
}

#[inline]
pub fn is_test_env() -> bool {
    env::var("MOON_TEST").is_ok() || env::var("STARBASE_TEST").is_ok()
}
