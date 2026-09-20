use moon_common::{is_daemon_env, is_test_env};
use moon_console::MoonReporter;
use moon_env_var::GlobalEnvBag;
use std::ffi::OsStr;

pub use starbase_process::*;

pub type Command = starbase_process::Command<MoonReporter>;

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
