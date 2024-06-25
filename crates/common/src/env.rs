use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

fn has_env_var(key: &str) -> bool {
    match env::var(key) {
        Ok(var) => !var.is_empty(),
        Err(_) => false,
    }
}

pub fn is_ci() -> bool {
    static CI_CACHE: OnceLock<bool> = OnceLock::new();

    *CI_CACHE.get_or_init(|| has_env_var("CI"))
}

pub fn is_docker() -> bool {
    static DOCKER_CACHE: OnceLock<bool> = OnceLock::new();

    *DOCKER_CACHE.get_or_init(|| {
        if PathBuf::from("/.dockerenv").exists() {
            return true;
        }

        match fs::read_to_string("/proc/self/cgroup") {
            Ok(contents) => contents.contains("docker"),
            Err(_) => false,
        }
    })
}

#[inline]
pub fn is_test_env() -> bool {
    static TEST_CACHE: OnceLock<bool> = OnceLock::new();

    *TEST_CACHE.get_or_init(|| {
        has_env_var("MOON_TEST") || has_env_var("STARBASE_TEST") || has_env_var("NEXTEST")
    })
}

#[inline]
pub fn is_formatted_output() -> bool {
    env::args().any(|arg| arg == "--json" || arg == "--dot")
}
