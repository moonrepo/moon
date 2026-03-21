use std::env;
use std::env::consts;
use std::fs;
use std::io::{self, IsTerminal};
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

    *CI_CACHE.get_or_init(|| {
        has_env_var("CI") ||
        has_env_var("CI_NAME") ||
        // Azure doesn't set the `CI` var
        has_env_var("AZURE_PIPELINES")
    })
}

pub fn is_ci_env() -> bool {
    is_ci() && !is_test_env()
}

pub fn is_devbox() -> bool {
    static DEVBOX_CACHE: OnceLock<bool> = OnceLock::new();

    *DEVBOX_CACHE.get_or_init(|| {
        for key in [
            "CODESPACES",
            "CODESPACE_NAME",
            "GITHUB_CODESPACES",
            "GITPOD_INSTANCE_ID",
            "DAYTONA_WORKSPACE_ID",
            "DEVPOD",
            "VSCODE_INJECTION",
            "REMOTE_CONTAINERS",
            "C9_HOSTNAME",
            "GOOGLE_CLOUD_PROJECT",
            "GCLOUD_PROJECT",
            "CHE_WORKSPACE_ID",
            "GL_WORKSPACE_ID",
            "CODESANDBOX_SSE",
            "REPL_ID",
        ] {
            if has_env_var(key) {
                return true;
            }
        }

        false
    })
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

pub fn is_headless() -> bool {
    static HEADLESS_CACHE: OnceLock<bool> = OnceLock::new();

    *HEADLESS_CACHE.get_or_init(|| !(io::stdin().is_terminal() && io::stdout().is_terminal()))
}

pub fn is_ssh() -> bool {
    static SSH_CACHE: OnceLock<bool> = OnceLock::new();

    *SSH_CACHE.get_or_init(|| has_env_var("SSH_CLIENT") || has_env_var("SSH_TTY"))
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

pub fn is_test_env() -> bool {
    static TEST_CACHE: OnceLock<bool> = OnceLock::new();

    *TEST_CACHE.get_or_init(|| {
        has_env_var("MOON_TEST") || has_env_var("STARBASE_TEST") || has_env_var("NEXTEST")
    })
}

pub fn is_remote() -> bool {
    is_ci() || is_ssh() || is_devbox() || is_headless()
}

pub fn is_local() -> bool {
    !is_remote()
}

#[inline]
pub fn is_formatted_output() -> bool {
    static FORMATTED_CACHE: OnceLock<bool> = OnceLock::new();

    *FORMATTED_CACHE.get_or_init(|| env::args().any(|arg| arg == "--json" || arg == "--dot"))
}
