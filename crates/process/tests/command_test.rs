use moon_process::{Command, CommandArg, Env, ShellType};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

mod command_arg {
    use super::*;

    #[test]
    fn converts_from_many_types() {
        assert_eq!(CommandArg::from("str").value, OsString::from("str"));
        assert_eq!(
            CommandArg::from(String::from("string")).value,
            OsString::from("string")
        );
        assert_eq!(
            CommandArg::from(OsStr::new("os_str")).value,
            OsString::from("os_str")
        );
        assert_eq!(
            CommandArg::from(PathBuf::from("path")).value,
            OsString::from("path")
        );
    }

    #[test]
    fn prefers_quoted_value() {
        let arg = CommandArg {
            quoted_value: Some(OsString::from("'value'")),
            value: OsString::from("value"),
        };

        assert_eq!(arg.as_os_str(), OsStr::new("'value'"));

        let arg = CommandArg::from("value");

        assert_eq!(arg.as_os_str(), OsStr::new("value"));
    }
}

mod env {
    use super::*;

    #[test]
    fn returns_value_per_variant() {
        assert_eq!(
            Env::Set(OsString::from("a")).get_value(),
            Some(&OsString::from("a"))
        );
        assert_eq!(
            Env::SetIfMissing(OsString::from("b")).get_value(),
            Some(&OsString::from("b"))
        );
        assert_eq!(Env::Unset.get_value(), None);
    }
}

mod args {
    use super::*;

    #[test]
    fn adds_and_lists_args() {
        let mut command = Command::new("git");
        command.arg("status").args(["--short", "--branch"]);

        assert_eq!(command.get_args_list(), ["status", "--short", "--branch"]);
    }

    #[test]
    fn arg_if_missing_skips_existing() {
        let mut command = Command::new("git");
        command.arg("status");
        command.arg_if_missing("status");
        command.arg_if_missing("--short");

        assert_eq!(command.get_args_list(), ["status", "--short"]);
    }

    #[test]
    fn contains_arg_checks_quoted_and_raw() {
        let mut command = Command::new("git");
        command.arg(CommandArg {
            quoted_value: Some(OsString::from("'with space'")),
            value: OsString::from("with space"),
        });

        assert!(command.contains_arg("with space"));
        assert!(command.contains_arg("'with space'"));
        assert!(!command.contains_arg("other"));
    }
}

mod envs {
    use super::*;

    #[test]
    fn sets_and_unsets_vars() {
        let mut command = Command::new("git");
        command.env("SET", "1");
        command.env_opt("OPT_NONE", None::<&str>);
        command.env_remove("REMOVED");

        assert_eq!(
            command.env.get(OsStr::new("SET")),
            Some(&Env::Set(OsString::from("1")))
        );
        assert_eq!(command.env.get(OsStr::new("OPT_NONE")), Some(&Env::Unset));
        assert_eq!(command.env.get(OsStr::new("REMOVED")), Some(&Env::Unset));
    }

    #[test]
    fn contains_env_includes_unset_vars() {
        let mut command = Command::new("git");
        command.env("SET", "1");
        command.env_remove("REMOVED");

        assert!(command.contains_env("SET"));
        assert!(command.contains_env("REMOVED"));
        assert!(!command.contains_env("MISSING"));
    }
}

mod paths {
    use super::*;

    #[test]
    fn appends_and_prepends_in_order() {
        let mut command = Command::new("git");
        command.append_paths(["/c"]);
        command.prepend_paths(["/a", "/b"]);

        assert_eq!(
            command.paths,
            [
                OsString::from("/a"),
                OsString::from("/b"),
                OsString::from("/c")
            ]
        );
    }
}

mod bin_name {
    use super::*;

    #[test]
    fn returns_binary_name() {
        assert_eq!(Command::new("git").get_bin_name(), "git");
        assert_eq!(Command::new_bin("cargo").get_bin_name(), "cargo");
    }

    #[test]
    fn returns_first_word_of_script() {
        assert_eq!(
            Command::new_script("git commit --allow-empty").get_bin_name(),
            "git"
        );
        assert_eq!(Command::new_script("solo").get_bin_name(), "solo");
    }
}

mod input {
    use super::*;

    #[test]
    fn tracks_input_and_size() {
        let mut command = Command::new("cat");

        assert!(!command.should_pass_stdin());

        command.input(["abc", "de"]);

        assert!(command.should_pass_stdin());
        assert_eq!(command.get_input_size(), 5);
    }
}

mod shells {
    use super::*;

    #[test]
    fn no_shell_removes_and_set_script_restores() {
        let mut command = Command::new("git");
        command.no_shell();

        assert!(command.shell.is_none());

        command.set_script("git status");

        assert!(command.shell.is_some());
    }
}

mod cache_key {
    use super::*;

    #[test]
    fn is_stable_across_env_insertion_order() {
        let mut a = Command::new("git");
        a.env("A", "1").env("B", "2");

        let mut b = Command::new("git");
        b.env("B", "2").env("A", "1");

        assert_eq!(a.get_cache_key(), b.get_cache_key());
    }

    #[test]
    fn does_not_collide_on_adjacent_values() {
        let mut a = Command::new("git");
        a.args(["ab", "c"]);

        let mut b = Command::new("git");
        b.args(["a", "bc"]);

        assert_ne!(a.get_cache_key(), b.get_cache_key());

        let mut a = Command::new("git");
        a.env("FOO", "BAR");

        let mut b = Command::new("git");
        b.env("FOOB", "AR");

        assert_ne!(a.get_cache_key(), b.get_cache_key());
    }

    #[test]
    fn distinguishes_env_variants() {
        let mut set = Command::new("git");
        set.env("KEY", "1");

        let mut set_if_missing = Command::new("git");
        set_if_missing.env_with_behavior("KEY", Env::SetIfMissing(OsString::from("1")));

        let mut unset = Command::new("git");
        unset.env_remove("KEY");

        let absent = Command::new("git");

        assert_ne!(set.get_cache_key(), set_if_missing.get_cache_key());
        assert_ne!(set.get_cache_key(), unset.get_cache_key());
        assert_ne!(unset.get_cache_key(), absent.get_cache_key());
    }

    #[test]
    fn changes_with_exe_args_cwd_and_input() {
        let base = Command::new("git").get_cache_key();

        let mut with_arg = Command::new("git");
        with_arg.arg("status");

        let mut with_cwd = Command::new("git");
        with_cwd.cwd("/tmp");

        let mut with_input = Command::new("git");
        with_input.input(["data"]);

        assert_ne!(with_arg.get_cache_key(), base);
        assert_ne!(with_cwd.get_cache_key(), base);
        assert_ne!(with_input.get_cache_key(), base);
        assert_ne!(Command::new("svn").get_cache_key(), base);
    }
}

mod command_line {
    use super::*;

    #[test]
    fn formats_binary_without_shell() {
        let mut command = Command::new("git");
        command.arg("status");

        assert_eq!(command.get_command_line(false, false), "git status");
    }

    #[test]
    fn formats_shell_wrapper_with_curly_quotes() {
        let mut command = Command::new("git");
        command.arg("status").set_shell(ShellType::Bash);

        let line = command.get_command_line(true, false);

        assert!(line.contains("-c “git status”"));
    }

    #[test]
    fn includes_input() {
        let mut command = Command::new("cat");
        command.no_shell();
        command.input(["abc"]);

        let line = command.get_command_line(true, true);

        assert!(line.ends_with("- abc"));
    }

    #[test]
    fn truncates_large_input() {
        let mut command = Command::new("cat");
        command.no_shell();
        command.input(["x".repeat(250)]);

        let line = command.get_command_line(true, true);

        assert!(line.contains("(truncated input, 250 total bytes)"));
    }
}

mod scripts {
    use super::*;

    #[test]
    fn returns_full_script() {
        assert_eq!(
            Command::new_script("git commit --allow-empty").get_script(),
            "git commit --allow-empty"
        );
        assert_eq!(Command::new("git").get_script(), "git");
    }
}
