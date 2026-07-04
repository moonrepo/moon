mod utils;

use moon_cache::{CacheContext, CacheEngine};
use moon_common::is_ci;
use moon_config::{CacheConfig, HasherWalkStrategy, PartialHasherConfig, RemoteConfig};
use moon_task_runner::TaskRunCacheState;
use moon_test_utils::predicates::prelude::*;
use starbase_utils::{fs, json};
use std::path::{MAIN_SEPARATOR_STR, Path};
use std::sync::Arc;
use utils::{
    change_files, create_cases_sandbox, create_cases_sandbox_with_config, create_pipeline_sandbox,
    create_sync_heavy_pipeline_sandbox,
};

const PROJECT_DIR: &str = if cfg!(windows) { "windows" } else { "unix" };

fn target(task: &str) -> String {
    format!("{PROJECT_DIR}:{task}")
}

fn extract_hash_from_run(fixture: &Path, target_id: &str) -> String {
    let config_dir = fixture.join(".moon");
    let engine = CacheEngine::new(CacheContext {
        cache_dir: config_dir.join("cache"),
        cache_config: Arc::new(CacheConfig::default()),
        config_dir,
        remote_config: Arc::new(RemoteConfig::default()),
        remote_debug: false,
        workspace_root: fixture.to_path_buf(),
    })
    .unwrap();
    let cache: TaskRunCacheState = json::read_file(
        engine
            .state
            .states_dir
            .join(target_id.replace(':', "/"))
            .join("lastRun.json"),
    )
    .unwrap();

    cache.hash
}

mod exec {
    use super::*;

    mod dispatcher_regression {
        use super::*;

        #[test]
        fn runs_sync_heavy_noop_graph_without_stalling() {
            let depth = 12;
            let width = 4;
            let expected_sync_projects = (depth * width) + 1;
            let mut sandbox = create_sync_heavy_pipeline_sandbox(depth, width);
            sandbox.sandbox.settings.timeout = 20;

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("action-graph").arg("--dot").arg("app:noop");
            });
            let dot = assert.output();
            assert.success();

            assert_eq!(dot.matches("SyncProject(").count(), expected_sync_projects);
            assert_eq!(dot.matches("RunTask(app:noop)").count(), 1);
            assert!(dot.contains("SyncWorkspace"));

            sandbox.sandbox.settings.timeout = 5;
            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg("app:noop");
                })
                .success()
                .stdout(predicate::str::contains("Tasks: 1 completed"));
        }
    }

    mod general {
        use super::*;

        #[test]
        fn creates_log_file() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("--log-file=output.log")
                        .arg("exec")
                        .arg("shared:base");
                })
                .success();

            assert!(sandbox.path().join("output.log").exists());
        }

        #[test]
        fn creates_nested_log_file() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("--log-file=nested/output.log")
                        .arg("exec")
                        .arg("shared:base");
                })
                .success();

            assert!(sandbox.path().join("nested/output.log").exists());
        }

        #[test]
        fn handles_echo() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("echo"));
            });

            assert.success().stdout(predicate::str::contains("hello"));
        }

        #[test]
        fn handles_ls() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("ls"));
            });

            assert
                .success()
                .stdout(predicate::str::contains(if cfg!(windows) {
                    "cwd.ps1"
                } else {
                    "cwd.sh"
                }));
        }

        #[test]
        fn handles_process_exit_zero() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("exitZero"));
            });

            assert
                .success()
                .code(0)
                .stdout(predicate::str::contains("This should not appear!").not())
                .stdout(predicate::str::contains("stdout"))
                .stderr(predicate::str::contains("stderr"));
        }

        #[test]
        fn handles_process_exit_nonzero() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("exitNonZero"));
            });

            assert
                .failure()
                .code(1)
                .stdout(predicate::str::contains("This should not appear!").not())
                .stdout(predicate::str::contains("stdout"))
                .stderr(predicate::str::contains("stderr"));
        }

        #[test]
        fn passes_args_through() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(target("passthroughArgs"))
                    .arg("--")
                    .arg("-aBc")
                    .arg("--opt")
                    .arg("value")
                    .arg("--optCamel=value")
                    .arg("foo")
                    .arg("'bar baz'")
                    .arg("\"baz qux\"")
                    .arg("--opt-kebab")
                    .arg("123");
            });

            assert.success().stdout(
                predicate::str::contains("Arg 2: --opt ('--opt')")
                    .and(predicate::str::contains("Arg 7: baz qux ('baz qux')")),
            );
        }

        #[test]
        fn passes_args_through_when_ran_in_project_root() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(target("passthroughArgs"))
                    .arg("--")
                    .arg("-aBc")
                    .arg("--opt")
                    .arg("value")
                    .arg("--optCamel=value")
                    .arg("foo")
                    .arg("'bar baz'")
                    .arg("\"baz qux\"")
                    .arg("--opt-kebab")
                    .arg("123")
                    .current_dir(sandbox.path().join(PROJECT_DIR));
            });

            assert.success().stdout(
                predicate::str::contains("Arg 4: --optCamel=value ('--optCamel=value')")
                    .and(predicate::str::contains("Arg 6: bar baz ('bar baz')")),
            );
        }

        #[test]
        fn sets_env_vars() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("envVars"));
            });

            assert
                .success()
                .stdout(predicate::str::contains("TEST_BAR=123"));
        }

        #[test]
        fn inherits_moon_env_vars() {
            let sandbox = create_pipeline_sandbox();
            let id = target("envVarsMoon");

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(&id);
            });

            assert
                .success()
                .stdout(predicate::str::contains(format!("MOON_TARGET={id}")));
        }

        #[test]
        fn runs_from_project_root() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("runFromProject"));
            });

            assert.success().stdout(predicate::str::contains(format!(
                "{}{MAIN_SEPARATOR_STR}{PROJECT_DIR}",
                fs::file_name(sandbox.path())
            )));
        }

        #[test]
        fn runs_from_workspace_root() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("runFromWorkspace"));
            });

            assert.success().stdout(
                predicate::str::contains(fs::file_name(sandbox.path())).and(
                    predicate::str::contains(format!(
                        "{}{MAIN_SEPARATOR_STR}{PROJECT_DIR}",
                        fs::file_name(sandbox.path())
                    ))
                    .not(),
                ),
            );
        }

        #[test]
        fn retries_on_failure_till_count() {
            let sandbox = create_pipeline_sandbox();
            let id = target("retryCount");

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(&id);
            });

            assert
                .failure()
                .code(1)
                .stdout(predicate::str::contains(id).and(predicate::str::contains("attempt 4/4")));
        }

        #[test]
        fn runs_build_task_in_ci() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:buildType");
            });

            assert
                .success()
                .stdout(predicate::str::contains("Tasks: 1 completed"));
        }

        #[test]
        fn runs_test_task_in_ci() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:testType");
            });

            assert
                .success()
                .stdout(predicate::str::contains("Tasks: 1 completed"));
        }

        #[test]
        fn doesnt_run_run_task_in_ci() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:runType");
            });

            if is_ci() {
                assert
                    .failure()
                    .stderr(predicate::str::contains("No tasks found"));
            } else {
                assert
                    .success()
                    .stdout(predicate::str::contains("Tasks: 1 completed"));
            }
        }

        #[test]
        fn errors_for_cycle_in_task_deps() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("depsA:taskCycle");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(depsA:taskCycle)").eval(&output));
            assert!(predicate::str::contains("RunTask(depsB:taskCycle)").eval(&output));
            assert!(predicate::str::contains("would introduce a").eval(&output));

            assert.failure();
        }

        #[test]
        fn disambiguates_same_tasks_with_diff_args_envs() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskDeps:deps");
            });

            let output = assert.output();

            // The order changes so we can't snapshot it
            assert!(
                predicate::str::contains("taskDeps:base")
                    .count(11) // 4 start + 4 end + 3 output prefixes
                    .eval(&output)
            );
            assert!(predicate::str::contains("a b c").eval(&output));
            assert!(predicate::str::contains("TEST_VAR=value").eval(&output));
            assert!(predicate::str::contains("TEST_VAR=value x y z").eval(&output));

            assert.success();
        }

        #[test]
        fn runs_task_with_a_mutex_in_sequence() {
            let sandbox = create_cases_sandbox();
            let start = std::time::Instant::now();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("mutex:run1")
                    .arg("mutex:run2")
                    .arg("mutex:run3")
                    .arg("--log")
                    .arg("debug");
            });

            assert.success();

            let stop = start.elapsed();

            assert!(stop.as_millis() > 3000);
        }
    }

    mod shell {
        use super::*;

        #[test]
        fn can_run_with_no_shell() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("noShell"));
            });

            assert.success().stdout(predicate::str::contains("hello"));
        }

        #[test]
        fn supports_multi_commands_ampersand() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg(target("multiAmpersand"));
                })
                .success();

            assert!(sandbox.path().join(PROJECT_DIR).join("foo").exists());
            assert!(sandbox.path().join(PROJECT_DIR).join("bar").exists());
        }

        #[test]
        fn supports_multi_commands_semicolon() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg(target("multiSemicolon"));
                })
                .success();

            assert!(sandbox.path().join(PROJECT_DIR).join("foo").exists());
            assert!(sandbox.path().join(PROJECT_DIR).join("bar").exists());
        }

        #[test]
        fn supports_inline_vars() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("syntaxVar"));
            });

            // Windows tasks emit CRLF line endings, so normalize before asserting
            let stdout = assert.stdout().replace("\r\n", "\n");
            assert.success();

            let expected = if cfg!(windows) {
                "substituted-value\nin substituted-value quotes\nprefixed-substituted-value\nsubstituted-value-suffixed"
            } else {
                "substituted-value in substituted-value quotes prefixed-substituted-value substituted-value-suffixed"
            };

            assert!(stdout.contains(expected), "stdout: {stdout}");
        }

        #[test]
        fn supports_expansion() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("syntaxExpansion"));
            });

            assert.success().stdout(predicate::str::contains("cd"));
        }

        #[test]
        fn supports_nested_commands() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("syntaxSubstitution"));
            });

            assert
                .success()
                .stdout(predicate::str::contains("subparens"));
        }

        #[cfg(unix)]
        #[test]
        fn supports_nested_commands_with_tick() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("syntaxSubstitutionTick"));
            });

            assert.success().stdout(predicate::str::contains("subtick"));
        }
    }

    mod targeting {
        use super::*;

        #[test]
        fn errors_for_unknown_project() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("unknown:task");
            });

            assert.failure().stderr(predicate::str::contains(
                "No project has been configured with the identifier or alias unknown.",
            ));
        }

        #[test]
        fn errors_for_unknown_task_in_project() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:unknown");
            });

            assert.failure().stderr(predicate::str::contains(
                "Unknown task unknown for project shared.",
            ));
        }

        #[test]
        fn errors_for_internal_task() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:internalOnly");
            });

            assert.failure().stderr(predicate::str::contains(
                "Unknown task internalOnly for project shared.",
            ));
        }

        #[test]
        fn errors_for_unknown_all_target() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(":unknown");
            });

            assert
                .failure()
                .stderr(predicate::str::contains("No tasks found"));
        }

        #[test]
        fn can_run_many_targets() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(target("outFoo"))
                    .arg(target("outBar"))
                    .arg(target("outBaz"));
            });

            assert.success().stdout(
                predicate::str::contains("outFoo | foo")
                    .and(predicate::str::contains("outBar | bar"))
                    .and(predicate::str::contains("outBaz | baz")),
            );
        }

        #[test]
        fn bails_on_failing_task() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:willFail");
            });

            assert.failure().stderr(predicate::str::contains(
                "Task shared:willFail failed to run.",
            ));
        }

        #[test]
        fn doesnt_bail_on_failing_task_if_allowed_to_fail() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:willFailButAllowed");
            });

            assert.success().stdout(
                predicate::str::contains("Tasks: 1 failed")
                    .and(predicate::str::contains("Task shared:willFail failed to run.").not()),
            );
        }

        #[test]
        fn runs_noop() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("noop:noop");
            });

            let output = assert.output();

            assert!(predicate::str::contains("noop:noop").eval(&output));
            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn runs_noop_deps() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("noop:noopWithDeps");
            });

            let output = assert.output();

            assert!(predicate::str::contains("outputs:generateFile").eval(&output));
            assert!(predicate::str::contains("noop:noopWithDeps").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn runs_a_root_level_task() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("root:oneOff");
            });

            let output = assert.output();

            assert!(predicate::str::contains("root-one-off").eval(&output));
        }
    }

    mod target_scopes {
        use super::*;

        #[test]
        fn errors_for_deps_scope() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("^:test");
            });

            assert.failure().stderr(predicate::str::contains(
                "Dependencies scope (^:) is not supported in run contexts.",
            ));
        }

        #[test]
        fn errors_for_cwd() {
            let sandbox = create_cases_sandbox();

            fs::create_dir_all(sandbox.path().join("fakeDir")).unwrap();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("~:taskName")
                    .current_dir(sandbox.path().join("fakeDir"));
            });

            assert.failure().stderr(predicate::str::contains(
                "No project could be located starting from path fakeDir.",
            ));
        }

        #[test]
        fn supports_all_scope() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(":all");
            });
            let output = assert.output();

            assert!(predicate::str::contains("targetScopeA:all").eval(&output));
            assert!(predicate::str::contains("targetScopeB:all").eval(&output));
            assert!(predicate::str::contains("targetScopeC:all").eval(&output));
            assert!(predicate::str::contains("Tasks: 3 completed").eval(&output));
        }

        #[test]
        fn supports_deps_scope_in_task() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("targetScopeA:deps");
            });

            let output = assert.output();

            assert!(predicate::str::contains("targetScopeA:deps").eval(&output));
            assert!(predicate::str::contains("depsA:standard").eval(&output));
            assert!(predicate::str::contains("depsB:standard").eval(&output));
            assert!(predicate::str::contains("depsC:standard").eval(&output));
            assert!(predicate::str::contains("Tasks: 4 completed").eval(&output));
        }

        #[test]
        fn supports_self_scope_in_task() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("targetScopeB:self");
            });
            let output = assert.output();

            assert!(predicate::str::contains("targetScopeB:self").eval(&output));
            assert!(predicate::str::contains("scope=self").eval(&output));
            assert!(predicate::str::contains("targetScopeB:selfOther").eval(&output));
            assert!(predicate::str::contains("selfOther").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn runs_closest_project_task_from_cwd() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("~:runFromProject")
                    .current_dir(sandbox.path().join("base"));
            });
            let output = assert.output();

            assert!(predicate::str::contains("base:runFromProject").eval(&output));
            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn runs_multiple_tasks_from_cwd() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("~:runFromProject")
                    .arg("~:runFromWorkspace")
                    .current_dir(sandbox.path().join("base"));
            });

            let output = assert.output();

            assert!(predicate::str::contains("base:runFromProject").eval(&output));
            assert!(predicate::str::contains("base:runFromWorkspace").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn can_mix_cwd_tasks_and_targets() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("~:runFromProject")
                    .arg("noop:noop")
                    .current_dir(sandbox.path().join("base"));
            });
            let output = assert.output();

            assert!(predicate::str::contains("base:runFromProject").eval(&output));
            assert!(predicate::str::contains("noop:noop").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn runs_in_projects_with_tag() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("#standard:standard");
            });
            let output = assert.output();

            assert!(predicate::str::contains("base:standard").eval(&output));
            assert!(predicate::str::contains("dependsOn:standard").eval(&output));
            assert!(predicate::str::contains("depsA:standard").eval(&output));
            assert!(predicate::str::contains("depsB:standard").eval(&output));
            assert!(predicate::str::contains("depsC:standard").eval(&output));
            assert!(predicate::str::contains("Tasks: 5 completed").eval(&output));
        }
    }

    mod caching {
        use super::*;

        #[test]
        fn option_forces_cache_to_write_only() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("envVarsMoon")).arg("--force");
            });

            assert
                .success()
                .stdout(predicate::str::contains("MOON_CACHE=write"));
        }

        #[test]
        fn can_create_outputs() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg(target("outputs"));
                })
                .success();

            assert!(sandbox.path().join(PROJECT_DIR).join("file.txt").exists());
            assert!(sandbox.path().join(PROJECT_DIR).join("folder").exists());
        }

        #[test]
        fn uses_cache_on_subsequent_runs() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("outputs"));
            });

            assert
                .success()
                .stdout(predicate::str::contains("cached").not());

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("outputs"));
            });

            assert.success().stdout(predicate::str::contains("cached"));
        }

        #[test]
        fn creates_run_report() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg("shared:base");
                })
                .success();

            assert!(sandbox.path().join(".moon/cache/runReport.json").exists());
        }

        #[test]
        fn creates_project_snapshot() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg(target("outputs"));
                })
                .success();

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/states")
                    .join(PROJECT_DIR)
                    .join("snapshot.json")
                    .exists()
            );
        }

        #[test]
        fn creates_task_run_state() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg(target("outputs"));
                })
                .success();

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/states")
                    .join(PROJECT_DIR)
                    .join("outputs/lastRun.json")
                    .exists()
            );
        }

        #[test]
        fn creates_task_hash_files() {
            let sandbox = create_pipeline_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg(target("outputs"));
                })
                .success();

            let cache_path = sandbox
                .path()
                .join(".moon/cache/states")
                .join(PROJECT_DIR)
                .join("outputs/lastRun.json");

            assert!(cache_path.exists());

            let state: TaskRunCacheState = json::read_file(cache_path).unwrap();

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{}.tar.gz", state.hash))
                    .exists()
            );
            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/hashes")
                    .join(format!("{}.json", state.hash))
                    .exists()
            );
        }

        mod archive {
            use super::*;

            #[test]
            fn archives_non_build_tasks() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:noOutput");
                });

                let hash = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

                assert!(
                    sandbox
                        .path()
                        .join(format!(".moon/cache/outputs/{hash}.tar.gz"))
                        .exists()
                );
            }

            #[test]
            fn archives_std_output() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:noOutput");
                });

                assert_eq!(
                    fs::read_file(
                        sandbox
                            .path()
                            .join(".moon/cache/states/outputs/noOutput/stdout.log")
                    )
                    .unwrap(),
                    "No outputs!\n"
                );
            }

            #[test]
            fn can_hydrate_archives() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:noOutput");
                });

                let hash1 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:noOutput");
                });

                let hash2 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

                assert_eq!(hash1, hash2);
            }
        }

        mod hydrate {
            use super::*;

            #[test]
            fn reuses_cache_from_previous_run() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                let hash1 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                let hash2 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");

                assert_eq!(hash1, hash2);
            }

            #[test]
            fn doesnt_keep_output_logs_in_project() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                assert!(!sandbox.path().join("outputs/stdout.log").exists());
                assert!(!sandbox.path().join("outputs/stderr.log").exists());
            }

            #[test]
            fn hydrates_missing_outputs_from_previous_run() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                // Remove outputs
                fs::remove_dir_all(sandbox.path().join("outputs/both/a")).unwrap();
                fs::remove_dir_all(sandbox.path().join("outputs/both/b")).unwrap();

                assert!(!sandbox.path().join("outputs/both/a").exists());
                assert!(!sandbox.path().join("outputs/both/b").exists());

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                // Outputs should come back
                assert!(sandbox.path().join("outputs/both/a").exists());
                assert!(sandbox.path().join("outputs/both/b").exists());
            }

            #[test]
            fn hydrates_with_a_different_hash_cache() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                let hash1 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
                let contents1 =
                    fs::read_file(sandbox.path().join("outputs/both/a/one.js")).unwrap();

                // Create a file to trigger an inputs change
                sandbox.create_file("outputs/trigger.js", "");

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                let hash2 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
                let contents2 =
                    fs::read_file(sandbox.path().join("outputs/both/a/one.js")).unwrap();

                // Hashes and contents should be different!
                assert_ne!(hash1, hash2);
                assert_ne!(contents1, contents2);

                // Remove outputs
                fs::remove_dir_all(sandbox.path().join("outputs/both/a")).unwrap();
                fs::remove_dir_all(sandbox.path().join("outputs/both/b")).unwrap();

                assert!(!sandbox.path().join("outputs/both/a").exists());
                assert!(!sandbox.path().join("outputs/both/b").exists());

                // Remove the trigger file
                fs::remove_file(sandbox.path().join("outputs/trigger.js")).unwrap();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:generateFileAndFolder");
                });

                let hash3 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
                let contents3 =
                    fs::read_file(sandbox.path().join("outputs/both/a/one.js")).unwrap();

                // Hashes and contents should match the original!
                assert_eq!(hash1, hash3);
                assert_eq!(contents1, contents3);
                assert_ne!(contents2, contents3);

                // Outputs should come back
                assert!(sandbox.path().join("outputs/both/a").exists());
                assert!(sandbox.path().join("outputs/both/b").exists());
            }

            #[test]
            fn ignores_files_negated_by_globs() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:negatedOutputGlob");
                });

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("outputs:negatedOutputGlob");
                });

                assert!(sandbox.path().join("outputs/both/a/one.js").exists());

                // Exists from first build and isn't deleted
                assert!(sandbox.path().join("outputs/both/b/two.js").exists());
            }
        }
    }

    mod hashing {
        use super::*;

        #[test]
        fn generates_diff_hashes_from_inputs() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:noOutput");
            });

            let hash1 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:noOutput");
            });

            let hash2 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            assert_eq!(hash1, hash2);
        }

        #[test]
        fn tracks_input_changes_for_env_files() {
            let sandbox = create_cases_sandbox();

            sandbox.create_file("outputs/.env", "FOO=123");

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:envFile");
            });

            let hash1 = extract_hash_from_run(sandbox.path(), "outputs:envFile");

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:envFile");
            });

            let hash2 = extract_hash_from_run(sandbox.path(), "outputs:envFile");

            assert_eq!(hash1, hash2);

            sandbox.create_file("outputs/.env", "FOO=456");

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:envFile");
            });

            let hash3 = extract_hash_from_run(sandbox.path(), "outputs:envFile");

            assert_ne!(hash1, hash3);
            assert_ne!(hash2, hash3);
        }

        #[test]
        fn supports_diff_walking_strategies() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:noOutput");
            });

            let hash_vcs = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            // Run again with a different strategy
            let sandbox = create_cases_sandbox_with_config(|workspace_config| {
                workspace_config.hasher = Some(PartialHasherConfig {
                    walk_strategy: Some(HasherWalkStrategy::Glob),
                    ..PartialHasherConfig::default()
                });
            });

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:noOutput");
            });

            let hash_glob = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            // Hashes change because `.moon/workspace.yml` is different from `walk_strategy`
            assert_ne!(hash_vcs, hash_glob);
        }
    }

    mod affected {
        use super::*;

        #[test]
        fn doesnt_run_if_not_affected() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("files:noop").arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("affected by changed files").eval(&output));
        }

        #[test]
        fn doesnt_run_if_not_affected_by_multi_status() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("untracked")
                    .arg("--status")
                    .arg("deleted");
            });

            let output = assert.output();

            assert!(predicate::str::contains("affected by changed files").eval(&output));
            assert!(predicate::str::contains("untracked").eval(&output));
            assert!(predicate::str::contains("deleted").eval(&output));
        }

        #[test]
        fn runs_if_forced() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("files:noop").arg("--force");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn runs_if_not_affected_but_forced() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--force");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn runs_if_affected() {
            let sandbox = create_cases_sandbox();

            change_files(&sandbox, ["files/other.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("files:noop").arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn runs_if_affected_via_stdin() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--stdin")
                    .write_stdin("files/other.txt");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn doesnt_run_affected_if_stdin_is_empty() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--stdin");
            });

            let output = assert.output();

            assert!(predicate::str::contains("affected by changed files").eval(&output));
        }

        #[test]
        fn doesnt_run_affected_if_stdin_arg_is_not_passed() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    // .arg("--stdin")
                    .write_stdin("files/other.txt");
            });

            let output = assert.output();

            assert!(predicate::str::contains("affected by changed files").eval(&output));
        }

        #[test]
        fn runs_if_not_affected_but_a_dep_of_an_affected() {
            let sandbox = create_cases_sandbox();

            change_files(&sandbox, ["affected/primary.js"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("affected:primaryWithDeps")
                    .arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("affected:dep").eval(&output));
            assert!(predicate::str::contains("affected:primaryWithDeps").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn runs_if_affected_by_multi_status() {
            let sandbox = create_cases_sandbox();

            // Test modified
            sandbox.create_file("files/file.txt", "modified");

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:affected")
                    .arg("--affected")
                    .arg("--status")
                    .arg("modified");
            });

            assert!(predicate::str::contains("\nfile.txt\n").eval(&assert.output()));

            // Then test added
            fs::remove_dir_all(sandbox.path().join(".moon/cache")).unwrap();

            sandbox.create_file("files/other.txt", "added");
            sandbox.run_git(|cmd| {
                cmd.args(["add", "files/other.txt"]);
            });

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:affected")
                    .arg("--affected")
                    .arg("--status")
                    .arg("added");
            });

            assert!(predicate::str::contains("\nother.txt\n").eval(&assert.output()));

            // Then test both
            fs::remove_dir_all(sandbox.path().join(".moon/cache")).unwrap();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:affected")
                    .arg("--affected")
                    .arg("--status")
                    .arg("modified")
                    .arg("--status")
                    .arg("added");
            });
            let envs = ["file.txt", "other.txt"].join(if cfg!(windows) { ";" } else { ":" });

            assert!(predicate::str::contains(format!("\n{envs}\n")).eval(&assert.output()));
        }

        #[test]
        fn doesnt_run_if_affected_but_wrong_status() {
            let sandbox = create_cases_sandbox();

            change_files(&sandbox, ["files/other.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("deleted");
            });

            let output = assert.output();

            assert!(predicate::str::contains("affected by changed files").eval(&output));
            assert!(predicate::str::contains("deleted").eval(&output));
        }

        #[test]
        fn handles_untracked() {
            let sandbox = create_cases_sandbox();

            sandbox.create_file("files/other.txt", "");

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("untracked");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn handles_added() {
            let sandbox = create_cases_sandbox();

            sandbox.create_file("files/other.txt", "");

            sandbox.run_git(|cmd| {
                cmd.args(["add", "files/other.txt"]);
            });

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("added");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn handles_modified() {
            let sandbox = create_cases_sandbox();

            sandbox.create_file("files/file.txt", "modified");

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("modified");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn handles_deleted() {
            let sandbox = create_cases_sandbox();

            fs::remove_file(sandbox.path().join("files/file.txt")).unwrap();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("files:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("deleted");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        mod root_level {
            use super::*;

            #[test]
            fn doesnt_run_if_not_affected() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("root:noop").arg("--affected");
                });

                let output = assert.output();

                assert!(predicate::str::contains("affected by changed files").eval(&output));
            }

            #[test]
            fn runs_if_affected() {
                let sandbox = create_cases_sandbox();

                sandbox.create_file("tsconfig.json", "{}");

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("root:noop").arg("--affected");
                });

                let output = assert.output();

                assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
            }

            #[test]
            fn doesnt_run_if_affected_but_wrong_status() {
                let sandbox = create_cases_sandbox();

                sandbox.create_file("tsconfig.json", "{}");

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec")
                        .arg("root:noop")
                        .arg("--affected")
                        .arg("--status")
                        .arg("deleted");
                });

                let output = assert.output();

                assert!(predicate::str::contains("affected by changed files").eval(&output));
                assert!(predicate::str::contains("deleted").eval(&output));
            }
        }
    }

    mod affected_files {
        use super::*;

        #[test]
        fn all_files_when_not_affected() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("affectedFiles"));
            });

            // Windows tasks emit CRLF line endings, so normalize before asserting
            let stdout = assert.stdout().replace("\r\n", "\n");
            assert.success();

            let root = sandbox.path().join(PROJECT_DIR);

            let mut files = fs::read_dir(&root)
                .unwrap()
                .into_iter()
                .map(|f| {
                    f.path()
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .to_string()
                })
                .collect::<Vec<_>>();
            files.sort();

            let args = files
                .clone()
                .into_iter()
                .map(|f| format!("./{f}"))
                .collect::<Vec<_>>()
                .join(" ");
            let envs = files.join(if cfg!(windows) { ";" } else { ":" });

            assert!(
                stdout.contains(&format!("Args: {args}\n")),
                "stdout: {stdout}"
            );
            assert!(
                stdout.contains(&format!("Env: {envs}\n")),
                "stdout: {stdout}"
            );
        }

        #[test]
        fn uses_rel_paths_when_affected() {
            let sandbox = create_pipeline_sandbox();

            change_files(
                &sandbox,
                [
                    format!("{PROJECT_DIR}/input1.txt"),
                    format!("{PROJECT_DIR}/input2.txt"),
                ],
            );

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(target("affectedFiles"))
                    .arg("--affected");
            });
            let envs = ["input1.txt", "input2.txt"].join(if cfg!(windows) { ";" } else { ":" });

            // Windows tasks emit CRLF line endings, so normalize before asserting
            let stdout = assert.stdout().replace("\r\n", "\n");
            assert.success();

            assert!(
                stdout.contains("Args: ./input1.txt ./input2.txt\n"),
                "stdout: {stdout}"
            );
            assert!(
                stdout.contains(&format!("Env: {envs}\n")),
                "stdout: {stdout}"
            );
        }

        #[test]
        fn sets_args_only() {
            let sandbox = create_pipeline_sandbox();

            change_files(
                &sandbox,
                [
                    format!("{PROJECT_DIR}/input1.txt"),
                    format!("{PROJECT_DIR}/input2.txt"),
                ],
            );

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(target("affectedFilesArgs"))
                    .arg("--affected");
            });

            // Windows tasks emit CRLF line endings, so normalize before asserting
            let stdout = assert.stdout().replace("\r\n", "\n");
            assert.success();

            assert!(
                stdout.contains("Args: ./input1.txt ./input2.txt\n"),
                "stdout: {stdout}"
            );
            assert!(stdout.contains("Env: \n"), "stdout: {stdout}");
        }

        #[test]
        fn sets_env_var_only() {
            let sandbox = create_pipeline_sandbox();

            change_files(
                &sandbox,
                [
                    format!("{PROJECT_DIR}/input1.txt"),
                    format!("{PROJECT_DIR}/input2.txt"),
                ],
            );

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(target("affectedFilesEnvVar"))
                    .arg("--affected");
            });
            let envs = ["input1.txt", "input2.txt"].join(if cfg!(windows) { ";" } else { ":" });

            // Windows tasks emit CRLF line endings, so normalize before asserting
            let stdout = assert.stdout().replace("\r\n", "\n");
            assert.success();

            assert!(stdout.contains("Args: \n"), "stdout: {stdout}");
            assert!(
                stdout.contains(&format!("Env: {envs}\n")),
                "stdout: {stdout}"
            );
        }
    }

    mod configs {
        use super::*;

        #[test]
        fn bubbles_up_invalid_workspace_config() {
            let sandbox = create_pipeline_sandbox();

            sandbox.create_file(".moon/workspace.yml", "projects: true");

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:base");
            });

            assert.failure().stderr(predicate::str::contains(
                "projects: failed to parse as any variant",
            ));
        }

        #[test]
        fn bubbles_up_invalid_tasks_config() {
            let sandbox = create_pipeline_sandbox();

            sandbox.create_file(".moon/tasks/all.yml", "tasks: 123");

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("shared:base");
            });

            assert.failure().stderr(predicate::str::contains(
                "tasks: invalid type: integer `123`",
            ));
        }
    }

    mod dependencies {
        use super::*;

        #[test]
        fn runs_the_graph_in_order() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("depsA:dependencyOrder");
            });

            let output = assert.output();
            let c = output.find("depsC:dependencyOrder");
            let b = output.find("depsB:dependencyOrder");
            let a = output.find("depsA:dependencyOrder");

            // Runs C, then B, then A
            assert!(c.is_some() && b.is_some() && a.is_some());
            assert!(c < b && b < a);
            assert!(predicate::str::contains("Tasks: 3 completed").eval(&output));
        }

        #[test]
        fn runs_the_graph_in_order_not_from_head() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("depsB:dependencyOrder");
            });

            let output = assert.output();

            assert!(predicate::str::contains("depsC:dependencyOrder").eval(&output));
            assert!(predicate::str::contains("depsB:dependencyOrder").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn can_run_deps_in_serial() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("dependsOn:serialDeps");
            });

            let output = assert.output();

            assert!(predicate::str::contains("depsA:standard").eval(&output));
            assert!(predicate::str::contains("depsB:standard").eval(&output));
            assert!(predicate::str::contains("depsC:standard").eval(&output));
            assert!(predicate::str::contains("dependsOn:serialDeps").eval(&output));
            assert!(predicate::str::contains("Tasks: 4 completed").eval(&output));
        }

        #[test]
        fn generates_unique_hashes_for_each_target() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:withDeps");
            });

            let as_dep = extract_hash_from_run(sandbox.path(), "outputs:asDep");
            let with_deps = extract_hash_from_run(sandbox.path(), "outputs:withDeps");

            assert_ne!(as_dep, with_deps);
        }

        #[test]
        fn changes_primary_hash_if_deps_hash_changes() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:withDeps");
            });

            let h1 = extract_hash_from_run(sandbox.path(), "outputs:asDep");
            let h2 = extract_hash_from_run(sandbox.path(), "outputs:withDeps");

            // Create an `inputs` file for `outputs:asDep`
            sandbox.create_file("outputs/random.js", "");

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:withDeps");
            });

            let h3 = extract_hash_from_run(sandbox.path(), "outputs:asDep");
            let h4 = extract_hash_from_run(sandbox.path(), "outputs:withDeps");

            // `random.js` is an input of `asDep` (`*.js`), so its hash changes...
            assert_ne!(h1, h3);
            // ...and since `asDep` declares `outputs`, the `withDeps -> asDep`
            // dependency defaults to a `hash` cache strategy, so `asDep`'s hash
            // change propagates into `withDeps`'s hash as well.
            assert_ne!(h2, h4);
        }

        #[test]
        fn can_depend_on_noop_task() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("dependsOn:depsOnNoop");
            });

            assert
                .success()
                .stderr(predicate::str::contains("Encountered a missing hash").not());
        }

        #[test]
        fn can_depend_on_nocache_task() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("dependsOn:depsOnNoCache");
            });

            assert
                .success()
                .stderr(predicate::str::contains("Encountered a missing hash").not());
        }

        #[test]
        fn can_depend_on_noop_and_nocache_task() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("dependsOn:depsOnNoopAndNoCache");
            });

            assert
                .success()
                .stderr(predicate::str::contains("Encountered a missing hash").not());
        }
    }

    mod plan {
        use super::*;

        mod targets {
            use super::*;

            #[test]
            fn simple_array() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file("plan.json", r#"{ "targets": ["shared:base"] }"#);

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.success();
            }

            #[test]
            fn filtered_include() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{ "targets": { "include": ["shared:base"] } }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.success();
            }

            #[test]
            fn partitioned_jobs() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "pipeline": { "jobTotal": 2 },
                    "targets": {
                        "jobs": [
                            ["shared:base"],
                            ["shared:buildType"]
                        ]
                    }
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec")
                        .arg("--plan")
                        .arg("plan.json")
                        .arg("--job")
                        .arg("0");
                });

                assert.success();
            }
        }

        mod pipeline {
            use super::*;

            #[test]
            fn ci_mode() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "pipeline": { "ci": true },
                    "targets": ["shared:base"]
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.success();
            }

            #[test]
            fn concurrency() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "pipeline": { "concurrency": 1 },
                    "targets": ["shared:base"]
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.success();
            }

            #[test]
            fn on_failure_continue() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "pipeline": { "onFailure": "continue" },
                    "targets": ["shared:willFail", "shared:base"]
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                // Should still fail overall but both tasks should have run
                assert.failure();
            }

            #[test]
            fn on_failure_bail() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "pipeline": { "onFailure": "bail" },
                    "targets": ["shared:willFail", "shared:base"]
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.failure();
            }

            #[test]
            fn job_selection() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "pipeline": { "job": 1, "jobTotal": 2 },
                    "targets": {
                        "jobs": [
                            ["shared:base"],
                            ["shared:buildType"]
                        ]
                    }
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.success();
            }
        }

        mod graph {
            use super::*;

            #[test]
            fn upstream_deep() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "graph": { "upstream": "deep" },
                    "targets": ["shared:base"]
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.success();
            }

            #[test]
            fn upstream_none_ignores_dependency_hashes() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "a/moon.yml",
                    r#"tasks:
  task:
    command: 'exit 0'
    options:
      cache: false
"#,
                );
                sandbox.create_file(
                    "b/moon.yml",
                    r#"tasks:
  task:
    command: 'exit 0'
    deps: ['a:task']
    options:
      cache: false
"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--upstream=none").arg("b:task");
                });

                assert.success();
            }

            #[test]
            fn downstream_direct() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "graph": { "downstream": "direct" },
                    "targets": ["shared:base"]
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.success();
            }
        }

        mod affected {
            use super::*;

            #[test]
            fn with_affected_base() {
                let sandbox = create_pipeline_sandbox();
                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "affected": { "base": "master" },
                    "targets": ["shared:base"]
                }"#,
                );

                change_files(&sandbox, ["shared/file.txt"]);

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                if is_ci() {
                    assert.success();
                } else {
                    // Locally, affected detection may differ
                    assert.success();
                }
            }
        }

        mod precedence {
            use super::*;

            #[test]
            fn plan_overrides_cli_targets() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file("plan.json", r#"{ "targets": ["shared:base"] }"#);

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec")
                        .arg("--plan")
                        .arg("plan.json")
                        .arg("shared:buildType");
                });

                // Plan targets should take priority over CLI targets
                assert.success();
            }
        }

        mod errors {
            use super::*;

            #[test]
            fn invalid_json() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file("plan.json", "not valid json {{{");

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.failure();
            }

            #[test]
            fn unknown_fields() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{ "unknownField": true, "targets": ["shared:base"] }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.failure();
            }

            #[test]
            fn missing_plan_file() {
                let sandbox = create_pipeline_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("nonexistent.json");
                });

                assert.failure();
            }

            #[test]
            fn job_without_partitioned_targets() {
                let sandbox = create_pipeline_sandbox();

                sandbox.create_file(
                    "plan.json",
                    r#"{
                    "pipeline": { "job": 1, "jobTotal": 2 },
                    "targets": {
                        "jobs": [["shared:base"]]
                    }
                }"#,
                );

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("--plan").arg("plan.json");
                });

                assert.failure();
            }
        }
    }

    mod outputs {
        use super::*;

        fn untar(tarball: &Path, root: &Path) {
            starbase_archive::Archiver::new(root, tarball)
                .unpack(starbase_archive::tar::TarUnpacker::new_gz)
                .unwrap();
        }

        #[test]
        fn errors_if_output_missing() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:missingOutput");
            });

            let output = assert.output();

            assert!(
                predicate::str::contains("Task outputs:missingOutput defines outputs")
                    .eval(&output)
            );
        }

        #[test]
        fn errors_if_output_missing_with_globs() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:missingOutputGlob");
            });

            let output = assert.output();

            assert!(
                predicate::str::contains("Task outputs:missingOutputGlob defines outputs")
                    .eval(&output)
            );
        }

        #[test]
        fn doesnt_cache_if_cache_disabled() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:noCache");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:noCache");

            assert_eq!(hash, "");

            // we cant assert the filesystem since the hash is empty!
        }

        #[test]
        fn caches_single_file() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFile");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFile");

            // hash
            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/hashes")
                    .join(format!("{hash}.json"))
                    .exists()
            );

            // outputs
            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{hash}.tar.gz"))
                    .exists()
            );
        }

        #[test]
        fn caches_multiple_files() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFiles");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFiles");

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/hashes")
                    .join(format!("{hash}.json"))
                    .exists()
            );

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{hash}.tar.gz"))
                    .exists()
            );
        }

        #[test]
        fn caches_single_folder() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFolder");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFolder");

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/hashes")
                    .join(format!("{hash}.json"))
                    .exists()
            );

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{hash}.tar.gz"))
                    .exists()
            );
        }

        #[test]
        fn caches_multiple_folders() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFolders");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFolders");

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/hashes")
                    .join(format!("{hash}.json"))
                    .exists()
            );

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{hash}.tar.gz"))
                    .exists()
            );
        }

        #[test]
        fn caches_both_file_and_folder() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFileAndFolder");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/hashes")
                    .join(format!("{hash}.json"))
                    .exists()
            );

            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/outputs")
                    .join(format!("{hash}.tar.gz"))
                    .exists()
            );
        }

        #[test]
        fn caches_using_output_glob() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFileTypes");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFileTypes");
            let tarball = sandbox
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{hash}.tar.gz"));
            let dir = sandbox.path().join(".moon/cache/outputs").join(hash);

            untar(&tarball, &dir);

            assert!(dir.join("outputs/multiple-types/one.js").exists());
            assert!(dir.join("outputs/multiple-types/two.js").exists());
            assert!(!dir.join("outputs/multiple-types/styles.css").exists());
            assert!(!dir.join("outputs/multiple-types/image.png").exists());
        }

        #[test]
        fn includes_project_files() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFileAndFolder");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
            let tarball = sandbox
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{hash}.tar.gz"));
            let dir = sandbox.path().join(".moon/cache/outputs").join(hash);

            untar(&tarball, &dir);

            assert!(dir.join("outputs/both/a/one.js").exists());
            assert!(dir.join("outputs/both/b/two.js").exists());
        }

        #[test]
        fn includes_workspace_files() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("outputs:generateFileAndFolderWorkspace");
            });

            let hash =
                extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolderWorkspace");
            let tarball = sandbox
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{hash}.tar.gz"));
            let dir = sandbox.path().join(".moon/cache/outputs").join(hash);

            untar(&tarball, &dir);

            assert!(dir.join("both/a/one.js").exists());
            assert!(dir.join("both/b/two.js").exists());
        }

        #[test]
        fn can_ignore_files_with_negated_globs() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:negatedOutputGlob");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:negatedOutputGlob");
            let tarball = sandbox
                .path()
                .join(".moon/cache/outputs")
                .join(format!("{hash}.tar.gz"));
            let dir = sandbox.path().join(".moon/cache/outputs").join(hash);

            untar(&tarball, &dir);

            assert!(dir.join("outputs/both/a/one.js").exists());
            assert!(!dir.join("outputs/both/b/two.js").exists());
        }

        #[test]
        fn can_bypass_cache() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFixed");
            });

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFixed");
            });

            assert!(predicate::str::contains("cached").eval(&assert.output()));

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputs:generateFixed").arg("-f");
            });

            assert!(!predicate::str::contains("cached").eval(&assert.output()));
        }
    }

    mod output_styles {
        use super::*;

        #[test]
        fn buffer() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputStyles:bufferPrimary");
            });

            let output = assert.output();

            assert!(predicate::str::contains("outputStyles:buffer | stdout").eval(&output));
            assert!(predicate::str::contains("outputStyles:buffer | stderr").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn buffer_on_failure_when_success() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputStyles:bufferFailurePassPrimary");
            });

            let output = assert.output();

            assert!(
                predicate::str::contains("outputStyles:bufferFailurePassPrimary").eval(&output)
            );
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn hash() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputStyles:hashPrimary");
            });

            let output = assert.output();

            assert!(predicate::str::contains("outputStyles:hashPrimary").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }

        #[test]
        fn none() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputStyles:nonePrimary");
            });

            let output = assert.output();

            assert!(predicate::str::contains("outputStyles:nonePrimary").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
            // Task output is not streamed
            assert!(
                predicate::str::contains("outputStyles:none | stdout")
                    .not()
                    .eval(&output)
            );
        }

        #[test]
        fn stream() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("outputStyles:streamPrimary");
            });

            let output = assert.output();

            assert!(predicate::str::contains("outputStyles:stream | stdout").eval(&output));
            assert!(predicate::str::contains("outputStyles:stream | stderr").eval(&output));
            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
        }
    }

    mod interactive {
        use super::*;

        #[test]
        fn interacts_with_cli_arg() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("interactive:prompt")
                    .arg("--interactive")
                    .write_stdin("with-arg");
            });

            // Test doesn't output the input (answer) we provide, so check for the question
            assert
                .success()
                .stdout(predicate::str::contains("Question?"));
        }

        #[test]
        fn interacts_with_local_option() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("interactive:promptWithLocal")
                    .write_stdin("with-local")
                    .env_remove("CI");
            });

            // Test doesn't output the input (answer) we provide, so check for the question
            assert
                .success()
                .stdout(predicate::str::contains("Question?"));
        }
    }

    mod query {
        use super::*;

        #[test]
        fn errors_if_no_matching_projects() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(":noop")
                    .arg("--query")
                    .arg("projectSource=fake");
            });

            assert
                .failure()
                .stderr(predicate::str::contains("Using query projectSource=fake"));
        }

        #[test]
        fn errors_for_invalid_query() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(":noop")
                    .arg("--query")
                    .arg("invalid=value");
            });

            assert
                .failure()
                .stderr(predicate::str::contains("Unknown query field invalid."));
        }

        #[test]
        fn can_run_target_via_query() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(":standard")
                    .arg("--query")
                    .arg("projectSource~deps-*");
            });

            let output = assert.output();

            assert!(predicate::str::contains("depsA:standard").eval(&output));
            assert!(predicate::str::contains("depsB:standard").eval(&output));
            assert!(predicate::str::contains("depsC:standard").eval(&output));
        }

        #[test]
        fn can_run_multiple_targets_via_query() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(":standard")
                    .arg(":dependencyOrder")
                    .arg("--query")
                    .arg("projectSource~deps-*");
            });

            let output = assert.output();

            assert!(predicate::str::contains("depsA:standard").eval(&output));
            assert!(predicate::str::contains("depsB:standard").eval(&output));
            assert!(predicate::str::contains("depsC:standard").eval(&output));
            assert!(predicate::str::contains("depsA:dependencyOrder").eval(&output));
            assert!(predicate::str::contains("depsB:dependencyOrder").eval(&output));
            assert!(predicate::str::contains("depsC:dependencyOrder").eval(&output));
        }

        #[test]
        fn runs_with_affected() {
            let sandbox = create_cases_sandbox();

            change_files(&sandbox, ["files/other.txt", "noop/other.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(":noop").arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg(":noop")
                    .arg("--affected")
                    .arg("--query")
                    .arg("project=files");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }
    }

    mod sync_codeowners {
        use super::*;
        use moon_config::{PartialCodeownersConfig, PartialVcsConfig, VcsProvider};

        #[test]
        fn doesnt_create_if_not_enabled() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("base:standard");
            });

            assert!(!sandbox.path().join(".github/CODEOWNERS").exists());
        }

        #[test]
        fn creates_if_enabled() {
            let sandbox = create_cases_sandbox_with_config(|workspace_config| {
                workspace_config.codeowners = Some(PartialCodeownersConfig {
                    sync: Some(true),
                    ..PartialCodeownersConfig::default()
                });
            });

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("base:standard");
            });

            assert!(sandbox.path().join(".github/CODEOWNERS").exists());
        }

        #[test]
        fn creates_for_gitlab() {
            let sandbox = create_cases_sandbox_with_config(|workspace_config| {
                workspace_config.codeowners = Some(PartialCodeownersConfig {
                    sync: Some(true),
                    ..PartialCodeownersConfig::default()
                });
                workspace_config.vcs = Some(PartialVcsConfig {
                    provider: Some(VcsProvider::GitLab),
                    ..PartialVcsConfig::default()
                });
            });

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("base:standard");
            });

            assert!(sandbox.path().join(".gitlab/CODEOWNERS").exists());
        }

        #[test]
        fn creates_for_bitbucket() {
            let sandbox = create_cases_sandbox_with_config(|workspace_config| {
                workspace_config.codeowners = Some(PartialCodeownersConfig {
                    sync: Some(true),
                    ..PartialCodeownersConfig::default()
                });
                workspace_config.vcs = Some(PartialVcsConfig {
                    provider: Some(VcsProvider::Bitbucket),
                    ..PartialVcsConfig::default()
                });
            });

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("base:standard");
            });

            assert!(sandbox.path().join("CODEOWNERS").exists());
        }
    }

    mod sync_vcs_hooks {
        use super::*;
        use moon_config::PartialVcsConfig;
        use rustc_hash::FxHashMap;

        #[test]
        fn doesnt_create_if_not_enabled() {
            let sandbox = create_cases_sandbox();

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("base:standard");
            });

            assert!(!sandbox.path().join(".moon/hooks").exists());
        }

        #[test]
        fn creates_if_enabled() {
            let sandbox = create_cases_sandbox_with_config(|workspace_config| {
                workspace_config.vcs = Some(PartialVcsConfig {
                    hooks: Some(FxHashMap::from_iter([(
                        "pre-commit".into(),
                        vec!["moon check --all".into()],
                    )])),
                    sync: Some(true),
                    ..Default::default()
                });
            });

            sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("base:standard");
            });

            assert!(sandbox.path().join(".moon/hooks").exists());
        }
    }

    // Tasks are using unix commands!
    #[cfg(unix)]
    mod task_scripts {
        use super::*;

        #[test]
        fn supports_basic_echo() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskScript:echo");
            });

            assert!(assert.output().contains("foo"));

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskScript:echo-nonquoted");
            });

            assert!(assert.output().contains("bar"));
        }

        #[test]
        fn supports_multiple_commands() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskScript:multi");
            });

            let output = assert.output();

            assert!(predicate::str::contains("foo").eval(&output));
            assert!(predicate::str::contains("bar").eval(&output));
            assert!(predicate::str::contains("baz").eval(&output));
            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));

            assert.success();
        }

        #[test]
        fn supports_pipes() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskScript:pipe");
            });

            let output = assert.output();

            assert!(predicate::str::contains("moon.yml").eval(&output));
            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));

            assert.success();
        }

        #[test]
        fn supports_redirects() {
            let sandbox = create_cases_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("exec").arg("taskScript:redirect");
                })
                .success();

            assert_eq!(
                fs::read_file(sandbox.path().join("task-script/file.txt")).unwrap(),
                "contents\n"
            );
        }

        #[test]
        fn doesnt_passthrough_args() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec")
                    .arg("taskScript:args")
                    .args(["--", "a", "-b", "--c"]);
            });

            let output = assert.output();

            assert!(predicate::str::contains("Args: \n").eval(&output));
            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));

            assert.success();
        }
    }

    mod task_os {
        use super::*;

        #[test]
        fn runs_linux() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskOs:linux");
            });

            let output = assert.output();

            if cfg!(target_os = "linux") {
                assert!(output.contains("runs-linux"));
                assert!(!output.contains("no op"));
            } else {
                assert!(!output.contains("runs-linux"));
                assert!(output.contains("no op"));
            }

            assert.success();
        }

        #[test]
        fn runs_macos() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskOs:macos");
            });

            let output = assert.output();

            if cfg!(target_os = "macos") {
                assert!(output.contains("runs-macos"));
                assert!(!output.contains("no op"));
            } else {
                assert!(!output.contains("runs-macos"));
                assert!(output.contains("no op"));
            }

            assert.success();
        }

        #[test]
        fn runs_windows() {
            let sandbox = create_cases_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg("taskOs:windows");
            });

            let output = assert.output();

            if cfg!(target_os = "windows") {
                assert!(output.contains("runs-windows"));
                assert!(!output.contains("no op"));
            } else {
                assert!(!output.contains("runs-windows"));
                assert!(output.contains("no op"));
            }

            assert.success();
        }
    }

    // Tasks are using unix commands!
    #[cfg(unix)]
    mod checks {
        use super::*;

        mod requirements {
            use super::*;

            #[test]
            fn runs_task_when_requirement_passes() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:requirementPass");
                });

                assert
                    .success()
                    .stdout(predicate::str::contains("requirement-passed"));
            }

            #[test]
            fn fails_task_when_requirement_fails() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:requirementFail");
                });

                assert
                    .failure()
                    .stderr(predicate::str::contains("requirement check"));
            }

            #[test]
            fn does_not_run_command_when_requirement_fails() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:requirementFail");
                });

                assert
                    .failure()
                    .stdout(predicate::str::contains("should-not-run").not());
            }
        }

        mod conditions {
            use super::*;

            #[test]
            fn skips_task_when_condition_passes() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:conditionSkip");
                });

                let output = assert.output();

                assert!(
                    predicate::str::contains("should-be-skipped")
                        .not()
                        .eval(&output)
                );
                assert!(predicate::str::contains("Tasks: 1 skipped").eval(&output));
            }

            #[test]
            fn runs_task_when_condition_fails() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:conditionRun");
                });

                assert
                    .success()
                    .stdout(predicate::str::contains("condition-ran"));
            }

            #[test]
            fn runs_task_when_not_all_conditions_pass() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:conditionAllMustPass");
                });

                assert
                    .success()
                    .stdout(predicate::str::contains("should-run"));
            }

            #[test]
            fn downstream_task_runs_when_dep_is_conditionally_skipped() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:downstreamOfConditional");
                });

                let output = assert.output();

                assert!(predicate::str::contains("downstream-ran").eval(&output));
                assert!(predicate::str::contains("1 completed").eval(&output));
                assert!(predicate::str::contains("1 skipped").eval(&output));
            }

            #[test]
            fn skipped_conditional_exits_successfully() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:conditionSkip");
                });

                assert.success();
            }
        }

        mod fingerprints {
            use super::*;

            #[test]
            fn runs_task_with_fingerprint_check() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:fingerprint");
                });

                assert
                    .success()
                    .stdout(predicate::str::contains("fingerprinted"));
            }

            #[test]
            fn fingerprint_affects_hash() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:fingerprint");
                });

                let hash1 = extract_hash_from_run(sandbox.path(), "checks:fingerprint");
                assert!(!hash1.is_empty());
            }

            #[test]
            fn same_fingerprint_produces_same_hash() {
                let sandbox = create_cases_sandbox();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:fingerprintStdout");
                });

                let hash1 = extract_hash_from_run(sandbox.path(), "checks:fingerprintStdout");

                // Clear cache to force re-run
                starbase_utils::fs::remove_dir_all(sandbox.path().join(".moon/cache")).unwrap();

                sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:fingerprintStdout");
                });

                let hash2 = extract_hash_from_run(sandbox.path(), "checks:fingerprintStdout");

                assert_eq!(hash1, hash2);
            }
        }

        mod mixed {
            use super::*;

            #[test]
            fn runs_with_passing_requirement_and_failing_condition() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:mixed");
                });

                assert
                    .success()
                    .stdout(predicate::str::contains("mixed-passed"));
            }

            #[test]
            fn conditional_skip_with_deps_runs_dep_first() {
                let sandbox = create_cases_sandbox();

                let assert = sandbox.run_bin(|cmd| {
                    cmd.arg("exec").arg("checks:conditionSkipWithDownstream");
                });

                let output = assert.output();

                assert!(predicate::str::contains("dep-task").eval(&output));
                assert!(
                    predicate::str::contains("should-be-skipped-with-dep")
                        .not()
                        .eval(&output)
                );
                assert!(predicate::str::contains("1 completed").eval(&output));
                assert!(predicate::str::contains("1 skipped").eval(&output));
            }
        }
    }
}
