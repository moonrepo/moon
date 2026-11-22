use moon_common::{is_daemon_env, is_test_env};
use moon_console::MoonReporter;
use moon_env_var::GlobalEnvBag;
use std::ffi::OsStr;
use tokio::time::timeout;

pub use starbase_process::*;

pub type Command = starbase_process::Command<MoonReporter>;

/// Wait for the running processes (tracked in the root process set) to
/// exit, but no longer than its grace period, unless it's 0, which waits
/// on them indefinitely.
pub async fn wait_for_running_to_shutdown() {
    let processes = ProcessSet::root();
    let grace = processes.grace();

    if grace.is_zero() {
        processes.wait_idle().await;
    } else {
        let _ = timeout(grace, processes.wait_idle()).await;
    }
}

pub trait CommandExt {
    fn create<T: AsRef<OsStr>>(bin: T) -> Command;
}

impl CommandExt for Command {
    fn create<T: AsRef<OsStr>>(bin: T) -> Command {
        let bag = GlobalEnvBag::instance();
        let mut cmd = Command::new(bin);

        cmd.debug = CommandDebug {
            env_key_prefixes: vec!["MOON_".into(), "PROTO_".into()],
            is_daemon_env: is_daemon_env(),
            is_test_env: is_test_env(),
            print_env: bag.should_debug_process_env(),
            print_input: bag.should_debug_process_input(),
            root_dir_env_key: Some("MOON_WORKSPACE_ROOT".into()),
        };

        cmd
    }
}
