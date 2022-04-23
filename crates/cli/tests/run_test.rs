use insta::assert_snapshot;
use moon_utils::path::replace_home_dir;
use moon_utils::test::{
    create_moon_command, get_assert_output, get_fixtures_dir, replace_fixtures_dir,
};
use predicates::prelude::*;

#[test]
fn errors_for_unknown_project() {
    let assert = create_moon_command("cases")
        .arg("run")
        .arg("unknown:test")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_unknown_task_in_project() {
    let assert = create_moon_command("cases")
        .arg("run")
        .arg("base:unknown")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

mod target_scopes {
    use super::*;

    #[test]
    fn errors_for_deps_scope() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("^:test")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn errors_for_self_scope() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("~:test")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn supports_all_scope() {
        let assert = create_moon_command("cases").arg("run").arg(":all").assert();
        let output = get_assert_output(&assert);

        // We cant use snapshots since it runs in parallel and the output changes
        assert!(predicate::str::contains("targetScopeA:all").eval(&output));
        assert!(predicate::str::contains("targetScopeB:all").eval(&output));
        assert!(predicate::str::contains("targetScopeC:all").eval(&output));
        assert!(predicate::str::contains("Tasks: 3 completed").eval(&output));
    }
}

mod node {
    use super::*;

    fn get_path_safe_output(assert: &assert_cmd::assert::Assert, fixtures_dir: &str) -> String {
        replace_home_dir(&replace_fixtures_dir(
            &get_assert_output(assert),
            &get_fixtures_dir(fixtures_dir),
        ))
    }

    #[test]
    fn runs_package_managers() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:npm")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
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

    #[test]
    fn passes_args_through() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:passthroughArgs")
            .arg("--")
            .arg("-aBc")
            .arg("--opt")
            .arg("value")
            .arg("--optCamel=value")
            .arg("foo")
            .arg("'bar baz'")
            .arg("--opt-kebab")
            .arg("123")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
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

    #[test]
    fn passes_args_through() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("system:passthroughArgs")
            .arg("--")
            .arg("-aBc")
            .arg("--opt")
            .arg("value")
            .arg("--optCamel=value")
            .arg("foo")
            .arg("'bar baz'")
            .arg("--opt-kebab")
            .arg("123")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
