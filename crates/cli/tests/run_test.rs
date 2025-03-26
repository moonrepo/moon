use moon_cache::CacheEngine;
use moon_config::{
    HasherWalkStrategy, PartialCodeownersConfig, PartialHasherConfig, PartialVcsConfig,
    PartialWorkspaceConfig, VcsProvider,
};
use moon_task_runner::TaskRunCacheState;
use moon_test_utils::{
    Sandbox, assert_debug_snapshot, assert_snapshot, create_sandbox_with_config,
    get_cases_fixture_configs, predicates::prelude::*,
};
use rustc_hash::FxHashMap;
use starbase_utils::json;
use std::fs;
use std::path::Path;

fn cases_sandbox() -> Sandbox {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    create_sandbox_with_config(
        "cases",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    )
}

fn cases_sandbox_with_config<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut PartialWorkspaceConfig),
{
    let (mut workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    callback(&mut workspace_config);

    create_sandbox_with_config(
        "cases",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    )
}

fn extract_hash_from_run(fixture: &Path, target_id: &str) -> String {
    let engine = CacheEngine::new(fixture).unwrap();
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

#[test]
fn errors_for_unknown_project() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("unknown:test");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn errors_for_unknown_task_in_project() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("base:unknown");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn errors_for_internal_task() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("base:internalOnly");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn errors_for_unknown_all_target() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg(":unknown");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn errors_for_cycle_in_task_deps() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("depsA:taskCycle");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn creates_run_report() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("base:standard");
    });

    assert!(sandbox.path().join(".moon/cache/runReport.json").exists());
}

#[test]
fn runs_with_shorthand_syntax() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    sandbox
        .run_moon(|cmd| {
            cmd.arg("base:standard");
        })
        .success();
}

#[test]
fn runs_with_shorthand_syntax_with_leading_option() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    sandbox
        .run_moon(|cmd| {
            cmd.arg("--force").arg("base:standard");
        })
        .success();
}

#[test]
fn bails_on_failing_task() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("states:willFail");
    });

    let output = assert.output();

    assert!(predicate::str::contains("Task states:willFail failed to run.").eval(&output));

    assert.failure();
}

#[test]
fn doesnt_bail_on_failing_task_if_allowed_to_fail() {
    let sandbox = cases_sandbox();
    sandbox.enable_git();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("states:willFailButAllowed");
    });

    let output = assert.output();

    assert!(!predicate::str::contains("Task states:willFail failed to run.").eval(&output));
    assert!(predicate::str::contains("Tasks: 1 failed").eval(&output));

    assert.success();
}

#[test]
fn disambiguates_same_tasks_with_diff_args_envs() {
    let sandbox = cases_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("taskDeps:deps");
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
    let sandbox = cases_sandbox();
    let start = std::time::Instant::now();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run")
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

#[cfg(not(windows))]
mod general {
    use super::*;
    use moon_config::PartialPipelineConfig;

    #[test]
    fn logs_command_for_project_root() {
        let sandbox = cases_sandbox_with_config(|cfg| {
            cfg.pipeline = Some(PartialPipelineConfig {
                log_running_command: Some(true),
                ..PartialPipelineConfig::default()
            });
        });
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:runFromProject");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn logs_command_for_workspace_root() {
        let sandbox = cases_sandbox_with_config(|cfg| {
            cfg.pipeline = Some(PartialPipelineConfig {
                log_running_command: Some(true),
                ..PartialPipelineConfig::default()
            });
        });
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:runFromWorkspace");
        });

        assert_snapshot!(assert.output());
    }
}

mod configs {
    use super::*;

    #[test]
    fn bubbles_up_invalid_workspace_config() {
        let sandbox = cases_sandbox();

        sandbox.create_file(".moon/workspace.yml", "projects: true");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:noop");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains(
                "projects: expected a list of globs, a map of projects, or both"
            )
            .eval(&output)
        );
    }

    #[test]
    fn bubbles_up_invalid_tasks_config() {
        let sandbox = cases_sandbox();

        sandbox.create_file(".moon/tasks.yml", "tasks: 123");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:noop");
        });

        let output = assert.output();

        assert!(predicate::str::contains("tasks: invalid type: integer `123`").eval(&output));
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

        sandbox
            .run_moon(|cmd| {
                cmd.arg("--logFile=nested/output.log")
                    .arg("run")
                    .arg("node:standard");
            })
            .debug();

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

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_the_graph_in_order_not_from_head() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("depsB:dependencyOrder");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_run_deps_in_serial() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("dependsOn:serialDeps");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn generates_unique_hashes_for_each_target() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:withDeps");
        });

        assert_debug_snapshot!([
            extract_hash_from_run(sandbox.path(), "outputs:asDep"),
            extract_hash_from_run(sandbox.path(), "outputs:withDeps")
        ]);
    }

    #[test]
    fn changes_primary_hash_if_deps_hash_changes() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:withDeps");
        });

        let h1 = extract_hash_from_run(sandbox.path(), "outputs:asDep");
        let h2 = extract_hash_from_run(sandbox.path(), "outputs:withDeps");

        // Create an `inputs` file for `outputs:asDep`
        sandbox.create_file("outputs/random.js", "");

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:withDeps");
        });

        assert_debug_snapshot!([
            h1,
            h2,
            extract_hash_from_run(sandbox.path(), "outputs:asDep"),
            extract_hash_from_run(sandbox.path(), "outputs:withDeps")
        ]);
    }

    #[test]
    fn can_depend_on_noop_task() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("dependsOn:depsOnNoop");
        });

        assert
            .success()
            .stderr(predicate::str::contains("Encountered a missing hash").not());
    }

    #[test]
    fn can_depend_on_nocache_task() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("dependsOn:depsOnNoCache");
        });

        assert
            .success()
            .stderr(predicate::str::contains("Encountered a missing hash").not());
    }

    #[test]
    fn can_depend_on_noop_and_nocache_task() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("dependsOn:depsOnNoopAndNoCache");
        });

        assert
            .success()
            .stderr(predicate::str::contains("Encountered a missing hash").not());
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

        assert_snapshot!(assert.output());
    }

    #[test]
    fn errors_for_cwd() {
        let sandbox = cases_sandbox();

        fs::create_dir(sandbox.path().join("fakeDir")).unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("taskName")
                .current_dir(sandbox.path().join("fakeDir"));
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn supports_all_scope() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg(":all");
        });
        let output = assert.output();

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

        let output = assert.output();

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
        let output = assert.output();

        assert!(predicate::str::contains("targetScopeB:self").eval(&output));
        assert!(predicate::str::contains("scope=self").eval(&output));
        assert!(predicate::str::contains("targetScopeB:selfOther").eval(&output));
        assert!(predicate::str::contains("selfOther").eval(&output));
        assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
    }

    #[test]
    fn runs_closest_project_task_from_cwd() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("runFromProject")
                .current_dir(sandbox.path().join("base"));
        });
        let output = assert.output();

        assert!(predicate::str::contains("base:runFromProject").eval(&output));
        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn runs_multiple_tasks_from_cwd() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("runFromProject")
                .arg("localOnly")
                // Allows local to run
                .env_remove("CI")
                .current_dir(sandbox.path().join("base"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("base:runFromProject").eval(&output));
        assert!(predicate::str::contains("base:localOnly").eval(&output));
        assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
    }

    #[test]
    fn can_mix_cwd_tasks_and_targets() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("runFromProject")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("#standard:standard");
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

mod hashing {
    use super::*;

    #[test]
    fn generates_diff_hashes_from_inputs() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:noOutput");
        });

        let hash1 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:noOutput");
        });

        let hash2 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn tracks_input_changes_for_env_files() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("outputs/.env", "FOO=123");

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:envFile");
        });

        let hash1 = extract_hash_from_run(sandbox.path(), "outputs:envFile");

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:envFile");
        });

        let hash2 = extract_hash_from_run(sandbox.path(), "outputs:envFile");

        assert_eq!(hash1, hash2);

        sandbox.create_file("outputs/.env", "FOO=456");

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:envFile");
        });

        let hash3 = extract_hash_from_run(sandbox.path(), "outputs:envFile");

        assert_ne!(hash1, hash3);
        assert_ne!(hash2, hash3);
    }

    #[test]
    fn supports_diff_walking_strategies() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:noOutput");
        });

        let hash_vcs = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

        // Run again with a different strategy
        let sandbox = cases_sandbox_with_config(|workspace_config| {
            workspace_config.hasher = Some(PartialHasherConfig {
                walk_strategy: Some(HasherWalkStrategy::Glob),
                ..PartialHasherConfig::default()
            });
        });
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:noOutput");
        });

        let hash_glob = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

        // Hashes change because `.moon/workspace.yml` is different from `walk_strategy`
        assert_debug_snapshot!(vec![hash_vcs, hash_glob]);
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:missingOutput");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains("Task outputs:missingOutput defines outputs").eval(&output)
        );
    }

    #[test]
    fn errors_if_output_missing_with_globs() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:missingOutputGlob");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains("Task outputs:missingOutputGlob defines outputs")
                .eval(&output)
        );
    }

    #[test]
    fn doesnt_cache_if_cache_disabled() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:noCache");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:noCache");

        assert_eq!(hash, "");

        // we cant assert the filesystem since the hash is empty!
    }

    #[test]
    fn caches_single_file() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFile");
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFiles");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFiles");

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
    fn caches_single_folder() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFolder");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFolder");

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
    fn caches_multiple_folders() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFolders");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFolders");

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
    fn caches_both_file_and_folder() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFileAndFolder");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");

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
    fn caches_using_output_glob() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFileTypes");
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFileAndFolder");
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFileAndFolderWorkspace");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolderWorkspace");
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:negatedOutputGlob");
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFixed");
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFixed");
        });

        assert!(predicate::str::contains("cached").eval(&assert.output()));

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFixed").arg("-u");
        });

        assert!(!predicate::str::contains("cached").eval(&assert.output()));
    }

    mod hydration {
        use super::*;

        #[test]
        fn reuses_cache_from_previous_run() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            let assert1 = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash1 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");

            let assert2 = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash2 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");

            assert_eq!(hash1, hash2);
            assert_snapshot!(assert1.output());
            assert_snapshot!(assert2.output());
        }

        #[test]
        fn doesnt_keep_output_logs_in_project() {
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

        #[test]
        fn hydrates_missing_outputs_from_previous_run() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            // Remove outputs
            fs::remove_dir_all(sandbox.path().join("outputs/both/a")).unwrap();
            fs::remove_dir_all(sandbox.path().join("outputs/both/b")).unwrap();

            assert!(!sandbox.path().join("outputs/both/a").exists());
            assert!(!sandbox.path().join("outputs/both/b").exists());

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            // Outputs should come back
            assert!(sandbox.path().join("outputs/both/a").exists());
            assert!(sandbox.path().join("outputs/both/b").exists());
        }

        #[test]
        fn hydrates_with_a_different_hash_cache() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash1 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
            let contents1 =
                fs::read_to_string(sandbox.path().join("outputs/both/a/one.js")).unwrap();

            // Create a file to trigger an inputs change
            sandbox.create_file("outputs/trigger.js", "");

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash2 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
            let contents2 =
                fs::read_to_string(sandbox.path().join("outputs/both/a/one.js")).unwrap();

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

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash3 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
            let contents3 =
                fs::read_to_string(sandbox.path().join("outputs/both/a/one.js")).unwrap();

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
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:negatedOutputGlob");
            });

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:negatedOutputGlob");
            });

            assert!(sandbox.path().join("outputs/both/a/one.js").exists());

            // Exists from first build and isn't deleted
            assert!(sandbox.path().join("outputs/both/b/two.js").exists());
        }
    }

    mod archiving {
        use super::*;

        #[test]
        fn archives_non_build_tasks() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
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
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            assert_eq!(
                fs::read_to_string(
                    sandbox
                        .path()
                        .join(".moon/cache/states/outputs/noOutput/stdout.log")
                )
                .unwrap(),
                "No outputs!"
            );
        }

        #[test]
        fn can_hydrate_archives() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            let hash1 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            let hash2 = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            assert_eq!(hash1, hash2);
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

        assert_snapshot!(assert.output());
    }

    #[test]
    fn runs_noop_deps() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("noop:noopWithDeps");
        });

        assert_snapshot!(assert.output());
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

        let output = assert.output();

        assert!(predicate::str::contains("root-one-off").eval(&output));
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

        assert_snapshot!(assert.output());
    }

    #[test]
    fn buffer_on_failure_when_success() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:bufferFailurePassPrimary");
        });

        assert_snapshot!(assert.output());
    }

    // #[cfg(not(windows))] // Different path output in snapshot
    // #[test]
    // fn buffer_on_failure_when_failure() {
    //     let sandbox = cases_sandbox();
    //     sandbox.enable_git();

    //     let assert = sandbox.run_moon(|cmd| {
    //         cmd.arg("run").arg("outputStyles:bufferFailureFailPrimary");
    //     });

    //     assert_snapshot!(assert.output());
    // }

    #[test]
    fn hash() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:hashPrimary");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn none() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:nonePrimary");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn stream() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:streamPrimary");
        });

        assert_snapshot!(assert.output());
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

        let output = assert.output();

        assert!(predicate::str::contains("not affected by touched files").eval(&output));
        assert!(predicate::str::contains("status all").eval(&output));
    }

    #[test]
    fn doesnt_run_if_not_affected_by_multi_status() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--status")
                .arg("untracked")
                .arg("--status")
                .arg("deleted");
        });

        let output = assert.output();

        assert!(predicate::str::contains("not affected by touched files").eval(&output));
        assert!(predicate::str::contains("status untracked, deleted").eval(&output));
    }

    #[test]
    fn runs_if_forced() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("files:noop").arg("--force");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn runs_if_not_affected_but_forced() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--force");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn runs_if_affected() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("files/other.txt", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("files:noop").arg("--affected");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn runs_if_not_affected_but_a_dep_of_an_affected() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("affected/primary.js", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        // Test modified
        sandbox.create_file("files/file.txt", "modified");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:affected")
                .arg("-u")
                .arg("--affected")
                .arg("--status")
                .arg("modified");
        });

        assert!(predicate::str::contains("\nfile.txt\n").eval(&assert.output()));

        // Then test added
        sandbox.create_file("files/other.txt", "added");
        sandbox.run_git(|cmd| {
            cmd.args(["add", "files/other.txt"]);
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:affected")
                .arg("-u")
                .arg("--affected")
                .arg("--status")
                .arg("added");
        });

        assert!(predicate::str::contains("\nother.txt\n").eval(&assert.output()));

        // Then test both
        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:affected")
                .arg("-u")
                .arg("--affected")
                .arg("--status")
                .arg("modified")
                .arg("--status")
                .arg("added");
        });

        assert!(predicate::str::contains("\nfile.txt,other.txt\n").eval(&assert.output()));
    }

    #[test]
    fn doesnt_run_if_affected_but_wrong_status() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("files/other.txt", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("files:noop")
                .arg("--affected")
                .arg("--status")
                .arg("deleted");
        });

        let output = assert.output();

        assert!(predicate::str::contains("not affected by touched files").eval(&output));
        assert!(predicate::str::contains("status deleted").eval(&output));
    }

    #[test]
    fn handles_untracked() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("files/other.txt", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("files/other.txt", "");

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

        let output = assert.output();

        assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
    }

    #[test]
    fn handles_modified() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("files/file.txt", "modified");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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

        let output = assert.output();

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

            let output = assert.output();

            assert!(predicate::str::contains("not affected by touched files").eval(&output));
            assert!(predicate::str::contains("status all").eval(&output));
        }

        #[test]
        fn runs_if_affected() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.create_file("tsconfig.json", "{}");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("root:noop").arg("--affected");
            });

            let output = assert.output();

            assert!(predicate::str::contains("Tasks: 1 completed").eval(&output));
        }

        #[test]
        fn doesnt_run_if_affected_but_wrong_status() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.create_file("tsconfig.json", "{}");

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run")
                    .arg("root:noop")
                    .arg("--affected")
                    .arg("--status")
                    .arg("deleted");
            });

            let output = assert.output();

            assert!(predicate::str::contains("not affected by touched files").eval(&output));
            assert!(predicate::str::contains("status deleted").eval(&output));
        }
    }
}

mod interactive {
    use super::*;

    #[test]
    fn interacts_with_cli_arg() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg(":noop")
                .arg("--query")
                .arg("projectSource=fake");
        });

        assert
            .success()
            .stdout(predicate::str::contains("Using query projectSource=fake"));
    }

    #[test]
    fn errors_for_invalid_query() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.create_file("files/other.txt", "");
        sandbox.create_file("noop/other.txt", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg(":noop").arg("--affected");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
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

    #[test]
    fn doesnt_create_if_not_enabled() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:standard");
        });

        assert!(!sandbox.path().join(".github/CODEOWNERS").exists());
    }

    #[test]
    fn creates_if_enabled() {
        let sandbox = cases_sandbox_with_config(|workspace_config| {
            workspace_config.codeowners = Some(PartialCodeownersConfig {
                sync_on_run: Some(true),
                ..PartialCodeownersConfig::default()
            });
        });

        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:standard");
        });

        assert!(sandbox.path().join(".github/CODEOWNERS").exists());
    }

    #[test]
    fn creates_for_gitlab() {
        let sandbox = cases_sandbox_with_config(|workspace_config| {
            workspace_config.codeowners = Some(PartialCodeownersConfig {
                sync_on_run: Some(true),
                ..PartialCodeownersConfig::default()
            });
            workspace_config.vcs = Some(PartialVcsConfig {
                provider: Some(VcsProvider::GitLab),
                ..PartialVcsConfig::default()
            });
        });

        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:standard");
        });

        assert!(sandbox.path().join(".gitlab/CODEOWNERS").exists());
    }

    #[test]
    fn creates_for_bitbucket() {
        let sandbox = cases_sandbox_with_config(|workspace_config| {
            workspace_config.codeowners = Some(PartialCodeownersConfig {
                sync_on_run: Some(true),
                ..PartialCodeownersConfig::default()
            });
            workspace_config.vcs = Some(PartialVcsConfig {
                provider: Some(VcsProvider::Bitbucket),
                ..PartialVcsConfig::default()
            });
        });

        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:standard");
        });

        assert!(sandbox.path().join("CODEOWNERS").exists());
    }
}

mod sync_vcs_hooks {
    use super::*;

    #[test]
    fn doesnt_create_if_not_enabled() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:standard");
        });

        assert!(!sandbox.path().join(".moon/hooks").exists());
    }

    #[test]
    fn creates_if_enabled() {
        let sandbox = cases_sandbox_with_config(|workspace_config| {
            workspace_config.vcs = Some(PartialVcsConfig {
                hooks: Some(FxHashMap::from_iter([(
                    "pre-commit".into(),
                    vec!["moon check --all".into()],
                )])),
                sync_hooks: Some(true),
                ..Default::default()
            });
        });

        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:standard");
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
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("taskScript:echo");
        });

        assert!(assert.output().contains("foo"));

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("taskScript:echo-nonquoted");
        });

        assert!(assert.output().contains("bar"));
    }

    #[test]
    fn supports_multiple_commands() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("taskScript:multi");
        });

        assert_snapshot!(assert.output());

        assert.success();
    }

    #[test]
    fn supports_pipes() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("taskScript:pipe");
        });

        assert_snapshot!(assert.output());

        assert.success();
    }

    #[test]
    fn supports_redirects() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox
            .run_moon(|cmd| {
                cmd.arg("run").arg("taskScript:redirect");
            })
            .success();

        sandbox.debug_files();

        assert_eq!(
            fs::read_to_string(sandbox.path().join("task-script/file.txt")).unwrap(),
            "contents\n"
        );
    }

    #[test]
    fn doesnt_passthrough_args() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("taskScript:args")
                .args(["--", "a", "-b", "--c"]);
        });

        assert_snapshot!(assert.output());

        assert.success();
    }
}

mod task_os {
    use super::*;

    #[test]
    fn runs_linux() {
        let sandbox = cases_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("taskOs:linux");
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
        let sandbox = cases_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("taskOs:macos");
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
        let sandbox = cases_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("taskOs:windows");
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
