use insta::assert_snapshot;
use moon_utils::path::replace_home_dir;
use moon_utils::test::{
    create_moon_command, get_assert_output, get_fixtures_dir, replace_fixtures_dir,
};

mod node {
    use super::*;

    fn get_path_safe_output(assert: &assert_cmd::assert::Assert, fixtures_dir: &str) -> String {
        replace_home_dir(&replace_fixtures_dir(
            &get_assert_output(assert),
            &get_fixtures_dir(fixtures_dir),
        ))
    }

    #[test]
    fn runs_standard_script() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:standard")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_cjs_files() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:cjs")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_mjs_files() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:mjs")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn supports_top_level_await() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:topLevelAwait")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_zero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:processExitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:processExitNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_code_zero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:exitCodeZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_code_nonzero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:exitCodeNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_throw_error() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:throwError")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, "cases"));
    }

    #[test]
    fn handles_unhandled_promise() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:unhandledPromise")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, "cases"));
    }
}

mod system {
    use super::*;

    #[test]
    fn handles_echo() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:echo")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_ls() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:ls")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_bash_script() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:bash")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
