mod utils;

use moon_common::is_ci;
use moon_task_runner::TaskRunCacheState;
use moon_test_utils2::predicates::prelude::*;
use starbase_utils::{fs, json};
use std::path::MAIN_SEPARATOR_STR;
use utils::{change_files, create_pipeline_sandbox};

const PROJECT_DIR: &str = if cfg!(windows) { "windows" } else { "unix" };

fn target(task: &str) -> String {
    format!("{PROJECT_DIR}:{task}")
}

mod exec {
    use super::*;

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

            assert
                .success()
                .stdout(predicate::str::contains(if cfg!(windows) {
                    "substituted-value\nin substituted-value quotes\nprefixed-substituted-value\nsubstituted-value-suffixed"
                } else {
                    "substituted-value in substituted-value quotes prefixed-substituted-value substituted-value-suffixed"
                }));
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
    }

    mod affected_files {
        use super::*;

        #[test]
        fn all_files_when_not_affected() {
            let sandbox = create_pipeline_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("exec").arg(target("affectedFiles"));
            });

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

            assert.success().stdout(
                predicate::str::contains(format!("Args: {args}\n"))
                    .and(predicate::str::contains(format!("Env: {envs}\n"))),
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

            assert.success().stdout(
                predicate::str::contains("Args: ./input1.txt ./input2.txt\n")
                    .and(predicate::str::contains(format!("Env: {envs}\n"))),
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

            assert.success().stdout(
                predicate::str::contains("Args: ./input1.txt ./input2.txt\n")
                    .and(predicate::str::contains("Env: \n")),
            );
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

            assert.success().stdout(
                predicate::str::contains("Args: \n")
                    .and(predicate::str::contains(format!("Env: {envs}\n"))),
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
}
