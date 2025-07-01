use moon_common::Id;
use moon_config::{PartialInheritedTasksConfig, PartialWorkspaceConfig, PartialWorkspaceProjects};
use moon_task_runner::TaskRunCacheState;
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, predicates::prelude::*,
};
use rustc_hash::FxHashMap;
use starbase_utils::json;

fn system_sandbox() -> Sandbox {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            (Id::raw("unix"), "unix".to_owned()),
            (Id::raw("windows"), "windows".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let tasks_config = PartialInheritedTasksConfig {
        // Avoid defaults in hashes or snapshots
        implicit_inputs: Some(vec![]),
        ..PartialInheritedTasksConfig::default()
    };

    let sandbox =
        create_sandbox_with_config("system", Some(workspace_config), None, Some(tasks_config));

    sandbox.enable_git();
    sandbox
}

#[cfg(not(windows))]
mod unix {
    use super::*;
    use std::fs;

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

        assert.code(1);
    }

    #[test]
    fn handles_process_exit_nonzero_inline() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:exitNonZeroInline");
        });

        assert_snapshot!(assert.output());

        assert.code(1);
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
                .arg("baz qux")
                .arg("--opt-kebab")
                .arg("123");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn passes_args_through_when_ran_in_project_root() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("passthroughArgs")
                .arg("--")
                .arg("-aBc")
                .arg("--opt")
                .arg("value")
                .arg("--optCamel=value")
                .arg("foo")
                .arg("'bar baz'")
                .arg("baz qux")
                .arg("--opt-kebab")
                .arg("123")
                .current_dir(sandbox.path().join("unix"));
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
    fn forces_cache_to_write_only() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:envVarsMoon").arg("--updateCache");
        });

        let output = assert.output();

        assert!(predicate::str::contains("MOON_CACHE=write").eval(&output));
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

    #[test]
    fn supports_multi_commands_ampersand() {
        let sandbox = system_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:multiAmpersand");
        });

        assert!(sandbox.path().join("unix/foo").exists());
        assert!(sandbox.path().join("unix/bar").exists());
    }

    #[test]
    fn supports_multi_commands_semicolon() {
        let sandbox = system_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:multiSemicolon");
        });

        assert!(sandbox.path().join("unix/foo").exists());
        assert!(sandbox.path().join("unix/bar").exists());
    }

    #[test]
    fn supports_inline_vars() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:syntaxVar");
        });

        let output = assert.output();

        assert!(predicate::str::contains("value").eval(&output));
    }

    // Works on macOS but not Linux
    #[cfg(target_os = "macos")]
    #[test]
    fn supports_expansion() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("unix:syntaxExpansion");
        });

        let output = assert.output();

        assert!(predicate::str::contains("cd").eval(&output));
    }

    mod caching {
        use super::*;

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
        fn creates_snapshot() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:outputs");
            });

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/states/unix/snapshot.json")
                    .exists()
            );
        }

        #[test]
        fn creates_run_state_cache() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:outputs");
            });

            let cache_path = sandbox
                .path()
                .join(".moon/cache/states/unix/outputs/lastRun.json");

            assert!(cache_path.exists());

            let state: TaskRunCacheState = json::read_file(cache_path).unwrap();

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{}.tar.gz", state.hash))
                    .exists()
            );
        }
    }

    mod affected_files {
        use super::*;

        #[test]
        fn all_files_when_not_affected() {
            let sandbox = system_sandbox();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("unix:affectedFiles");
            });

            let mut files = fs::read_dir(sandbox.path().join("unix"))
                .unwrap()
                .map(|f| {
                    f.unwrap()
                        .path()
                        .strip_prefix(sandbox.path().join("unix"))
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned()
                })
                .collect::<Vec<_>>();
            files.sort();

            let args = files
                .clone()
                .into_iter()
                .map(|f| "./".to_owned() + &f)
                .collect::<Vec<_>>()
                .join(" ");
            let envs = files.join(",");

            let output = assert.output();

            assert!(predicate::str::contains(format!("Args: {args}\n")).eval(&output));
            assert!(predicate::str::contains(format!("Env: {envs}\n")).eval(&output));
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
            assert!(predicate::str::contains("Env: input1.txt,input2.txt").eval(&output));
        }

        #[test]
        fn sets_args_only() {
            let sandbox = system_sandbox();

            sandbox.create_file("unix/input1.txt", "");
            sandbox.create_file("unix/input2.txt", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("unix:affectedFilesArgs")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: ./input1.txt ./input2.txt\n").eval(&output));
            assert!(predicate::str::contains("Env: \n").eval(&output));
        }

        #[test]
        fn sets_env_var_only() {
            let sandbox = system_sandbox();

            sandbox.create_file("unix/input1.txt", "");
            sandbox.create_file("unix/input2.txt", "");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("unix:affectedFilesEnvVar")
                    .arg("--affected");
            });
            let output = assert.output();

            assert!(predicate::str::contains("Args: \n").eval(&output));
            assert!(predicate::str::contains("Env: input1.txt,input2.txt\n").eval(&output));
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::env;

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

        assert.code(1);
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
    fn forces_cache_to_write_only() {
        let sandbox = system_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("windows:envVarsMoon")
                .arg("--updateCache");
        });

        let output = assert.output();

        assert!(predicate::str::contains("MOON_CACHE=write").eval(&output));
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

        // This fails in CI because of ps1 privileges
        if env::var("CI").is_err() {
            assert!(predicate::str::contains("windows:foo | foo").eval(&output));
            assert!(predicate::str::contains("windows:bar | bar").eval(&output));
            assert!(predicate::str::contains("windows:baz | baz").eval(&output));
        }
    }

    mod caching {
        use super::*;

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
        fn creates_snapshot() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("windows:outputs");
            });

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/states/windows/snapshot.json")
                    .exists()
            );
        }

        #[test]
        fn creates_run_state_cache() {
            let sandbox = system_sandbox();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("windows:outputs");
            });

            let cache_path = sandbox
                .path()
                .join(".moon/cache/states/windows/outputs/lastRun.json");

            assert!(cache_path.exists());

            let state: TaskRunCacheState = json::read_file(cache_path).unwrap();

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{}.tar.gz", state.hash))
                    .exists()
            );
        }
    }
}
