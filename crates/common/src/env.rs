use std::env;
use std::env::consts;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

fn has_env_var(key: &str) -> bool {
    match env::var(key) {
        Ok(var) => !var.is_empty(),
        Err(_) => false,
    }
}

fn has_proc_config(path: &str, value: &str) -> bool {
    match fs::read_to_string(path) {
        Ok(contents) => contents.to_lowercase().contains(value),
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

        has_proc_config("/proc/self/cgroup", "docker")
    })
}

pub fn is_wsl() -> bool {
    static WSL_CACHE: OnceLock<bool> = OnceLock::new();

    *WSL_CACHE.get_or_init(|| {
        if consts::OS != "linux" || is_docker() {
            return false;
        }

        if has_proc_config("/proc/sys/kernel/osrelease", "microsoft") {
            return true;
        }

        has_proc_config("/proc/version", "microsoft")
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

#[inline]
pub fn get_env_home(key: &str) -> Option<PathBuf> {
    match env::var(key) {
        Ok(value) => {
            let path = PathBuf::from(value);

            Some(if path.is_absolute() {
                path
            } else {
                env::current_dir().unwrap().join(path)
            })
        }
        Err(_) => None,
    }
}

#[inline]
pub fn get_resolved_env_home<F: FnOnce(PathBuf) -> PathBuf>(key: &str, fallback: F) -> PathBuf {
    match get_env_home(key) {
        Some(path) => path,
        None => fallback(dirs::home_dir().unwrap()),
    }
}
