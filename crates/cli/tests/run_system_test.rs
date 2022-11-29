use moon_config::{RunnerConfig, WorkspaceConfig, WorkspaceProjects};
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, predicates::prelude::*, Sandbox,
};
use moon_utils::string_vec;
use rustc_hash::FxHashMap;

fn system_sandbox() -> Sandbox {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("unix".to_owned(), "unix".to_owned()),
            ("windows".to_owned(), "windows".to_owned()),
        ])),
        runner: RunnerConfig {
            // Avoid these in hashes or snapshots
            implicit_inputs: string_vec![],
            ..RunnerConfig::default()
        },
        ..WorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config("system", Some(&workspace_config), None, None);

    sandbox.enable_git();
    sandbox
}

#[cfg(not(windows))]
mod unix {
    use super::*;

    #[test]
    fn handles_echo() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:echo");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_ls() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:ls");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_bash_script() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:bash");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_zero() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:exitZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:exitNonZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn passes_args_through() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("unix:passthroughArgs")
                .arg("--")
                .arg("-aBc")
                .arg("--opt")
                .arg("value")
                .arg("--optCamel=value")
                .arg("foo")
                .arg("'bar baz'")
                .arg("--opt-kebab")
                .arg("123");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn sets_env_vars() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:envVars");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn inherits_moon_env_vars() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:envVarsMoon");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_from_project_root() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:runFromProject");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_from_workspace_root() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:runFromWorkspace");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn retries_on_failure_till_count() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:retryCount");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_run_many_targets() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("unix:foo")
                .arg("unix:bar")
                .arg("unix:baz");
        });

        let output = assert.output();

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
            let sandbox = system_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:outputs");
            });

            assert_snapshot!(assert.output());

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:outputs");
            });

            assert_snapshot!(assert.output());
        }

        #[test]
        fn creates_runfile() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:outputs");
            });

            assert!(sandbox
                .path()
                .join(".moon/cache/states/unix/runfile.json")
                .exists());
        }

        #[tokio::test]
        async fn creates_run_state_cache() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:outputs");
            });

            let cache_path = sandbox
                .path()
                .join(".moon/cache/states/unix/outputs/lastRun.json");

            assert!(cache_path.exists());

            let state = RunTargetState::load(cache_path, 0).await.unwrap();

            assert_snapshot!(fs::read_to_string(
                sandbox
                    .path()
                    .join(format!(".moon/cache/hashes/{}.json", state.hash))
            )
            .unwrap());

            assert!(sandbox
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{}.tar.gz", state.hash))
                .exists());
            assert!(sandbox
                .path()
                .join(".moon/cache/states/unix/outputs/stdout.log")
                .exists());
            assert!(sandbox
                .path()
                .join(".moon/cache/states/unix/outputs/stderr.log")
                .exists());
        }
    }

    mod affected_files {
        use super::*;

        #[test]
        fn uses_dot_when_not_affected() {
            let sandbox = system_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:affectedFiles");
            });
            let output = assert.output();

            assert!(predicate::str::contains("Args: .\n").eval(&output));
        }

        #[test]
        fn uses_rel_paths_when_affected() {
            let sandbox = system_sandbox();

            sandbox.create_file("unix/input1.txt", "");
            sandbox.create_file("unix/input2.txt", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:affectedFiles").arg("--affected");
            });
            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.txt ./input2.txt").eval(&output));
        }

        #[test]
        fn sets_env_var() {
            let sandbox = system_sandbox();

            sandbox.create_file("unix/input1.txt", "");
            sandbox.create_file("unix/input2.txt", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("unix:affectedFilesEnvVar")
                    .arg("--affected");
            });
            let output = assert.output();

            assert!(
                predicate::str::contains("MOON_AFFECTED_FILES=./input1.txt,./input2.txt")
                    .eval(&output)
            );
        }
    }
}

#[cfg(windows)]
mod system_windows {
    use super::*;

    #[test]
    fn runs_bat_script() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:bat");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_zero() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:exitZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:exitNonZero");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn passes_args_through() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("windows:passthroughArgs")
                .arg("--")
                .arg("-aBc")
                .arg("--opt")
                .arg("value")
                .arg("--optCamel=value")
                .arg("foo")
                .arg("'bar baz'")
                .arg("--opt-kebab")
                .arg("123");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn sets_env_vars() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:envVars");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn inherits_moon_env_vars() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:envVarsMoon");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_from_project_root() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:runFromProject");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_from_workspace_root() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:runFromWorkspace");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn retries_on_failure_till_count() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("windows:retryCount");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_run_many_targets() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("windows:foo")
                .arg("windows:bar")
                .arg("windows:baz");
        });

        let output = assert.output();

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
            let sandbox = system_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("windows:outputs");
            });

            assert_snapshot!(assert.output());

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("windows:outputs");
            });

            assert_snapshot!(assert.output());
        }

        #[test]
        fn creates_runfile() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("windows:outputs");
            });

            assert!(sandbox
                .path()
                .join(".moon/cache/states/windows/runfile.json")
                .exists());
        }

        #[tokio::test]
        async fn creates_run_state_cache() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("windows:outputs");
            });

            let cache_path = sandbox
                .path()
                .join(".moon/cache/states/windows/outputs/lastRun.json");

            assert!(cache_path.exists());

            let state = RunTargetState::load(cache_path, 0).await.unwrap();

            assert_snapshot!(fs::read_to_string(
                sandbox
                    .path()
                    .join(format!(".moon/cache/hashes/{}.json", state.hash))
            )
            .unwrap());

            assert!(sandbox
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{}.tar.gz", state.hash))
                .exists());
            assert!(sandbox
                .path()
                .join(".moon/cache/states/windows/outputs/stdout.log")
                .exists());
            assert!(sandbox
                .path()
                .join(".moon/cache/states/windows/outputs/stderr.log")
                .exists());
        }
    }
}
