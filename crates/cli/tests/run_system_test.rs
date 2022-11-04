mod utils;

use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox_with_git, get_assert_output};
use utils::get_path_safe_output;

#[cfg(not(windows))]
mod unix {
    use super::*;

    #[test]
    fn handles_echo() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:echo")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_ls() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:ls")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_bash_script() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:bash")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_zero() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:exitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:exitNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn passes_args_through() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:passthroughArgs")
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

    #[test]
    fn sets_env_vars() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:envVars")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn inherits_moon_env_vars() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:envVarsMoon")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_project_root() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:runFromProject")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_workspace_root() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:runFromWorkspace")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn retries_on_failure_till_count() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:retryCount")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    mod passthrough {
        use super::*;

        #[test]
        fn doesnt_pass_to_non_primary() {
            let fixture = create_sandbox_with_git("system");

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("passthroughArgs:b")
                .arg("--")
                .arg("-aBc")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }
    }
}

#[cfg(windows)]
mod system_windows {
    use super::*;

    #[test]
    fn runs_bat_script() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:bat")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn handles_process_exit_zero() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:exitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:exitNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn passes_args_through() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:passthroughArgs")
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

    #[test]
    fn sets_env_vars() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:envVars")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn inherits_moon_env_vars() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:envVarsMoon")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_project_root() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:runFromProject")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_workspace_root() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:runFromWorkspace")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn retries_on_failure_till_count() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:retryCount")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
