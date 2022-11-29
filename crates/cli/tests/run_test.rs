mod utils;

use moon_cache::CacheEngine;
use moon_config::WorkspaceConfig;
use moon_test_utils::{
    assert_snapshot, create_sandbox, create_sandbox_with_config, get_assert_output,
    get_cases_fixture_configs, predicates::prelude::*, Sandbox,
};
use moon_utils::path::standardize_separators;
use std::fs;
use std::path::{Path, PathBuf};
use utils::get_path_safe_output;

fn cases_sandbox() -> Sandbox {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    )
}

fn cases_sandbox_with_config<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut WorkspaceConfig),
{
    let (mut workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    callback(&mut workspace_config);

    create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    )
}

async fn extract_hash_from_run(fixture: &Path, target_id: &str) -> String {
    let engine = CacheEngine::load(fixture).await.unwrap();
    let cache = engine.cache_run_target_state(target_id).await.unwrap();

    cache.hash
}

#[test]
fn errors_for_unknown_project() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("unknown:test");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_unknown_task_in_project() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("base:unknown");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_unknown_all_target() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg(":unknown");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_cycle_in_task_deps() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("depsA:taskCycle");
    });

    assert_snapshot!(get_assert_output(&assert));
}

#[cfg(not(windows))]
mod general {
    use super::*;

    #[test]
    fn logs_command_for_project_root() {
        let sandbox = cases_sandbox_with_config(|cfg| {
            cfg.runner.log_running_command = true;
        });
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:runFromProject");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn logs_command_for_workspace_root() {
        let sandbox = cases_sandbox_with_config(|cfg| {
            cfg.runner.log_running_command = true;
        });
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:runFromWorkspace");
        });

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod configs {
    use super::*;

    #[test]
    fn bubbles_up_invalid_workspace_config() {
        let sandbox = create_sandbox("cases");

        fs::write(sandbox.path().join(".moon/workspace.yml"), "projects: true").unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:noop");
        });

        assert_snapshot!(standardize_separators(get_path_safe_output(
            &assert,
            &PathBuf::from("./fake/path")
        )));
    }

    #[test]
    fn bubbles_up_invalid_global_project_config() {
        let sandbox = create_sandbox("cases");

        fs::write(sandbox.path().join(".moon/project.yml"), "tasks: 123").unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:noop");
        });

        assert_snapshot!(standardize_separators(get_path_safe_output(
            &assert,
            &PathBuf::from("./fake/path")
        )));
    }

    #[test]
    fn bubbles_up_invalid_project_config() {
        let sandbox = create_sandbox("cases");

        fs::write(
            sandbox.path().join("base/moon.yml"),
            "project:\n  type: library",
        )
        .unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:noop");
        });

        assert_snapshot!(standardize_separators(get_path_safe_output(
            &assert,
            &PathBuf::from("./fake/path")
        )));
    }
}

mod logs {
    use super::*;

    #[test]
    fn creates_log_file() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("--logFile=output.log")
                .arg("run")
                .arg("node:standard");
        });

        let output_path = sandbox.path().join("output.log");

        assert!(output_path.exists());
    }

    #[test]
    fn creates_nested_log_file() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("--logFile=nested/output.log")
                .arg("run")
                .arg("node:standard");
        });

        let output_path = sandbox.path().join("nested/output.log");

        assert!(output_path.exists());
    }
}

mod dependencies {
    use super::*;

    #[test]
    fn runs_the_graph_in_order() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("depsA:dependencyOrder");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_the_graph_in_order_not_from_head() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("depsB:dependencyOrder");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn can_run_deps_in_serial() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("dependsOn:serialDeps");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[tokio::test]
    async fn generates_unique_hashes_for_each_target() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:withDeps");
        });

        assert_eq!(
            extract_hash_from_run(sandbox.path(), "outputs:asDep").await,
            "92c5b8c6dceccedc0547032c9eeb5be64d225545ac679c6b1bb7d41baf892d77"
        );
        assert_eq!(
            extract_hash_from_run(sandbox.path(), "outputs:withDeps").await,
            "3d4fef133338cff776bd701538bf4861c92dce2e1f1282feb35d42a1b13c4b3b"
        );
    }

    #[tokio::test]
    async fn changes_primary_hash_if_deps_hash_changes() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:withDeps");
        });

        assert_eq!(
            extract_hash_from_run(sandbox.path(), "outputs:asDep").await,
            "92c5b8c6dceccedc0547032c9eeb5be64d225545ac679c6b1bb7d41baf892d77"
        );
        assert_eq!(
            extract_hash_from_run(sandbox.path(), "outputs:withDeps").await,
            "3d4fef133338cff776bd701538bf4861c92dce2e1f1282feb35d42a1b13c4b3b"
        );

        // Create an `inputs` file for `outputs:asDep`
        fs::write(sandbox.path().join("outputs/random.js"), "").unwrap();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:withDeps");
        });

        assert_eq!(
            extract_hash_from_run(sandbox.path(), "outputs:asDep").await,
            "296411f059717096bc78f8bd1a5f33019de11bd57480c900d9d3a25ea5441ca1"
        );
        assert_eq!(
            extract_hash_from_run(sandbox.path(), "outputs:withDeps").await,
            "b7af1e952f2146ce128b74809598233fdc17cc0dadd3e863edaf1cb8c69f019b"
        );
    }
}

mod target_scopes {
    use super::*;

    #[test]
    fn errors_for_deps_scope() {
        let sandbox = cases_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("^:test");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn errors_for_self_scope() {
        let sandbox = cases_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("~:test");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn supports_all_scope() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg(":all");
        });
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:all").eval(&output));
        assert!(predicate::str::contains("targetScopeB:all").eval(&output));
        assert!(predicate::str::contains("targetScopeC:all").eval(&output));
        assert!(predicate::str::contains("Tasks: 3 completed").eval(&output));
    }

    #[test]
    fn supports_deps_scope_in_task() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("targetScopeA:deps");
        });
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:deps").eval(&output));
        assert!(predicate::str::contains("depsA:standard").eval(&output));
        assert!(predicate::str::contains("depsB:standard").eval(&output));
        assert!(predicate::str::contains("depsC:standard").eval(&output));
        assert!(predicate::str::contains("Tasks: 4 completed").eval(&output));
    }

    #[test]
    fn supports_self_scope_in_task() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("targetScopeB:self");
        });
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeB:self").eval(&output));
        assert!(predicate::str::contains("scope=self").eval(&output));
        assert!(predicate::str::contains("targetScopeB:selfOther").eval(&output));
        assert!(predicate::str::contains("selfOther").eval(&output));
        assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
    }
}

mod outputs {
    use super::*;

    #[tokio::test]
    async fn errors_if_output_missing() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:missingOutput");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Target outputs:missingOutput defines the output unknown, but this output does not exist after being ran.").eval(&output));
    }

    #[tokio::test]
    async fn doesnt_cache_if_cache_disabled() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:noCache");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:noCache").await;

        assert_eq!(hash, "");

        // we cant assert the filesystem since the hash is empty!
    }

    #[tokio::test]
    async fn caches_single_file() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFile");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFile").await;

        // hash
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_multiple_files() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFiles");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFiles").await;

        // hash
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_single_folder() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFolder");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFolder").await;

        // hash
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_multiple_folders() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFolders");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFolders").await;

        // hash
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_both_file_and_folder() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFileAndFolder");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder").await;

        // hash
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash))
            .exists());
    }

    #[tokio::test]
    async fn caches_output_logs_in_tarball() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFile");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFile").await;
        let tarball = sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{}.tar.gz", hash));
        let dir = sandbox.path().join(".moon/cache/outputs").join(hash);

        moon_archive::untar(tarball, &dir, None).unwrap();

        assert!(dir.join("stdout.log").exists());
        assert!(dir.join("stderr.log").exists());
    }

    mod hydration {
        use super::*;
        use moon_test_utils::pretty_assertions::assert_eq;

        #[tokio::test]
        async fn reuses_cache_from_previous_run() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            let assert1 = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash1 =
                extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder").await;

            let assert2 = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash2 =
                extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder").await;

            assert_eq!(hash1, hash2);
            assert_snapshot!(get_assert_output(&assert1));
            assert_snapshot!(get_assert_output(&assert2));
        }

        #[tokio::test]
        async fn doesnt_keep_output_logs_in_project() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            assert!(!sandbox.path().join("outputs/stdout.log").exists());
            assert!(!sandbox.path().join("outputs/stderr.log").exists());
        }

        #[tokio::test]
        async fn hydrates_missing_outputs_from_previous_run() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            // Remove outputs
            fs::remove_dir_all(sandbox.path().join("outputs/esm")).unwrap();
            fs::remove_dir_all(sandbox.path().join("outputs/lib")).unwrap();

            assert!(!sandbox.path().join("outputs/esm").exists());
            assert!(!sandbox.path().join("outputs/lib").exists());

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            // Outputs should come back
            assert!(sandbox.path().join("outputs/esm").exists());
            assert!(sandbox.path().join("outputs/lib").exists());
        }

        #[tokio::test]
        async fn hydrates_with_a_different_hash_cache() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash1 =
                extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder").await;
            let contents1 = fs::read_to_string(sandbox.path().join("outputs/lib/one.js")).unwrap();

            // Create a file to trigger an inputs change
            fs::write(sandbox.path().join("outputs/trigger.js"), "").unwrap();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash2 =
                extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder").await;
            let contents2 = fs::read_to_string(sandbox.path().join("outputs/lib/one.js")).unwrap();

            // Hashes and contents should be different!
            assert_ne!(hash1, hash2);
            assert_ne!(contents1, contents2);

            // Remove outputs
            fs::remove_dir_all(sandbox.path().join("outputs/esm")).unwrap();
            fs::remove_dir_all(sandbox.path().join("outputs/lib")).unwrap();

            assert!(!sandbox.path().join("outputs/esm").exists());
            assert!(!sandbox.path().join("outputs/lib").exists());

            // Remove the trigger file
            fs::remove_file(sandbox.path().join("outputs/trigger.js")).unwrap();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash3 =
                extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder").await;
            let contents3 = fs::read_to_string(sandbox.path().join("outputs/lib/one.js")).unwrap();

            // Hashes and contents should match the original!
            assert_eq!(hash1, hash3);
            assert_eq!(contents1, contents3);
            assert_ne!(contents2, contents3);

            // Outputs should come back
            assert!(sandbox.path().join("outputs/esm").exists());
            assert!(sandbox.path().join("outputs/lib").exists());
        }
    }
}

mod noop {
    use super::*;

    #[test]
    fn runs_noop() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("noop:noop");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_noop_deps() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("noop:noopWithDeps");
        });

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod root_level {
    use super::*;

    #[test]
    fn runs_a_task() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:oneOff");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("root one off").eval(&output));
    }
}

mod output_styles {
    use super::*;

    #[test]
    fn buffer() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:bufferPrimary");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn buffer_on_failure_when_success() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:bufferFailurePassPrimary");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[cfg(not(windows))] // Different path output in snapshot
    #[test]
    fn buffer_on_failure_when_failure() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:bufferFailureFailPrimary");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn hash() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:hashPrimary");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn none() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:nonePrimary");
        });

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn stream() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:streamPrimary");
        });

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod reports {
    use super::*;

    #[test]
    fn doesnt_create_a_report_by_default() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:base");
        });

        assert!(!sandbox.path().join(".moon/cache/runReport.json").exists());
    }

    #[test]
    fn creates_report_when_option_passed() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:base").arg("--report");
        });

        assert!(sandbox.path().join(".moon/cache/runReport.json").exists());
    }
}

mod affected {
    use super::*;

    #[test]
    fn doesnt_run_if_not_affected() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("files:noop").arg("--affected");
        });

        let output = get_assert_output(&assert);

        assert!(
            predicate::str::contains("Target(s) files:noop not affected by touched files")
                .eval(&output)
        );
    }

    #[test]
    fn runs_if_affected() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        fs::write(sandbox.path().join("files/other.txt"), "").unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("files:noop").arg("--affected");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn doesnt_run_if_affected_but_wrong_status() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        fs::write(sandbox.path().join("files/other.txt"), "").unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--status")
                .arg("deleted");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains(
            "Target(s) files:noop not affected by touched files (using status deleted)"
        )
        .eval(&output));
    }

    #[test]
    fn handles_untracked() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        fs::write(sandbox.path().join("files/other.txt"), "").unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--status")
                .arg("untracked");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn handles_added() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        fs::write(sandbox.path().join("files/other.txt"), "").unwrap();

        sandbox.run_git(|cmd| {
            cmd.args(["add", "files/other.txt"]);
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--status")
                .arg("added");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn handles_modified() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        fs::write(sandbox.path().join("files/file.txt"), "modified").unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--status")
                .arg("modified");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn handles_deleted() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        fs::remove_file(sandbox.path().join("files/file.txt")).unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--status")
                .arg("deleted");
        });

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    mod root_level {
        use super::*;

        #[test]
        fn doesnt_run_if_not_affected() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("root:noop").arg("--affected");
            });

            let output = get_assert_output(&assert);

            assert!(
                predicate::str::contains("Target(s) root:noop not affected by touched files")
                    .eval(&output)
            );
        }

        #[test]
        fn runs_if_affected() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            fs::write(sandbox.path().join("tsconfig.json"), "{}").unwrap();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("root:noop").arg("--affected");
            });

            let output = get_assert_output(&assert);

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn doesnt_run_if_affected_but_wrong_status() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            fs::write(sandbox.path().join("tsconfig.json"), "{}").unwrap();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("root:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("deleted");
            });

            let output = get_assert_output(&assert);

            assert!(predicate::str::contains(
                "Target(s) root:noop not affected by touched files (using status deleted)"
            )
            .eval(&output));
        }
    }
}
