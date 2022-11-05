mod utils;

use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox_with_git, get_assert_output};
use predicates::prelude::*;
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

    #[test]
    fn can_run_many_targets() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("unix:foo")
            .arg("unix:bar")
            .arg("unix:baz")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("unix:foo | foo").eval(&output));
        assert!(predicate::str::contains("unix:bar | bar").eval(&output));
        assert!(predicate::str::contains("unix:baz | baz").eval(&output));
    }

    mod caching {
        use super::*;
        use moon_cache::RunTargetState;
        use std::fs;

        #[test]
        fn uses_cache_on_subsequent_runs() {
            let fixture = create_sandbox_with_git("system");

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("unix:outputs")
                .assert();

            assert_snapshot!(get_assert_output(&assert));

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("unix:outputs")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }

        #[test]
        fn creates_runfile() {
            let fixture = create_sandbox_with_git("system");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("unix:outputs")
                .assert();

            assert!(fixture
                .path()
                .join(".moon/cache/states/unix/runfile.json")
                .exists());
        }

        #[tokio::test]
        async fn creates_run_state_cache() {
            let fixture = create_sandbox_with_git("system");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("unix:outputs")
                .assert();

            let cache_path = fixture
                .path()
                .join(".moon/cache/states/unix/outputs/lastRun.json");

            assert!(cache_path.exists());

            let state = RunTargetState::load(cache_path, 0).await.unwrap();

            assert_snapshot!(fs::read_to_string(
                fixture
                    .path()
                    .join(format!(".moon/cache/hashes/{}.json", state.hash))
            )
            .unwrap());

            assert!(fixture
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{}.tar.gz", state.hash))
                .exists());
            assert!(fixture
                .path()
                .join(".moon/cache/states/unix/outputs/stdout.log")
                .exists());
            assert!(fixture
                .path()
                .join(".moon/cache/states/unix/outputs/stderr.log")
                .exists());
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

    #[test]
    fn can_run_many_targets() {
        let fixture = create_sandbox_with_git("system");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("windows:foo")
            .arg("windows:bar")
            .arg("windows:baz")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("windows:foo | foo").eval(&output));
        assert!(predicate::str::contains("windows:bar | bar").eval(&output));
        assert!(predicate::str::contains("windows:baz | baz").eval(&output));
    }

    mod caching {
        use super::*;
        use moon_cache::RunTargetState;
        use std::fs;

        #[test]
        fn uses_cache_on_subsequent_runs() {
            let fixture = create_sandbox_with_git("system");

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("windows:outputs")
                .assert();

            assert_snapshot!(get_assert_output(&assert));

            let assert = create_moon_command(fixture.path())
                .arg("run")
                .arg("windows:outputs")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }

        #[test]
        fn creates_runfile() {
            let fixture = create_sandbox_with_git("system");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("windows:outputs")
                .assert();

            assert!(fixture
                .path()
                .join(".moon/cache/states/windows/runfile.json")
                .exists());
        }

        #[tokio::test]
        async fn creates_run_state_cache() {
            let fixture = create_sandbox_with_git("system");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("windows:outputs")
                .assert();

            let cache_path = fixture
                .path()
                .join(".moon/cache/states/windows/outputs/lastRun.json");

            assert!(cache_path.exists());

            let state = RunTargetState::load(cache_path, 0).await.unwrap();

            assert_snapshot!(fs::read_to_string(
                fixture
                    .path()
                    .join(format!(".moon/cache/hashes/{}.json", state.hash))
            )
            .unwrap());

            assert!(fixture
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{}.tar.gz", state.hash))
                .exists());
            assert!(fixture
                .path()
                .join(".moon/cache/states/windows/outputs/stdout.log")
                .exists());
            assert!(fixture
                .path()
                .join(".moon/cache/states/windows/outputs/stderr.log")
                .exists());
        }
    }
}
