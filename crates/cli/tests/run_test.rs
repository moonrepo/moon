use moon_cache::CacheEngine;
use moon_config::{HasherWalkStrategy, WorkspaceConfig};
use moon_test_utils::{
    assert_debug_snapshot, assert_snapshot, create_sandbox_with_config, get_cases_fixture_configs,
    predicates::{self, prelude::*},
    Sandbox,
};
use std::fs;
use std::path::Path;

fn cases_sandbox() -> Sandbox {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    )
}

fn cases_sandbox_with_config<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut WorkspaceConfig),
{
    let (mut workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    callback(&mut workspace_config);

    create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    )
}

fn extract_hash_from_run(fixture: &Path, target_id: &str) -> String {
    let engine = CacheEngine::load(fixture).unwrap();
    let cache = engine.cache_run_target_state(target_id).unwrap();

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
        cmd.arg("run").arg("base:base");
    });

    assert!(sandbox.path().join(".moon/cache/runReport.json").exists());
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

        assert_snapshot!(assert.output());
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

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn bubbles_up_invalid_tasks_config() {
        let sandbox = cases_sandbox();

        sandbox.create_file(".moon/tasks.yml", "tasks: 123");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:noop");
        });

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn bubbles_up_invalid_project_config() {
        let sandbox = cases_sandbox();

        sandbox.create_file("base/moon.yml", "project:\n  type: library");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("base:noop");
        });

        assert_snapshot!(assert.output_standardized());
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
    fn errors_for_self_scope() {
        let sandbox = cases_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("~:test");
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
            workspace_config.hasher.walk_strategy = HasherWalkStrategy::Glob;
        });
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:noOutput");
        });

        let hash_glob = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

        assert_eq!(hash_vcs, hash_glob);
    }
}

mod outputs {
    use super::*;

    #[test]
    fn errors_if_output_missing() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:missingOutput");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Target outputs:missingOutput defines the output unknown, but this output does not exist after being ran.").eval(&output));
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
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{hash}.json"))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{hash}.tar.gz"))
            .exists());
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
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{hash}.json"))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{hash}.tar.gz"))
            .exists());
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
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{hash}.json"))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{hash}.tar.gz"))
            .exists());
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
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{hash}.json"))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{hash}.tar.gz"))
            .exists());
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
        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{hash}.json"))
            .exists());

        // outputs
        assert!(sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{hash}.tar.gz"))
            .exists());
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

        moon_archive::untar(tarball, &dir, None).unwrap();

        assert!(dir.join("build/one.js").exists());
        assert!(dir.join("build/two.js").exists());
        assert!(!dir.join("build/styles.css").exists());
        assert!(!dir.join("build/image.png").exists());
    }

    #[test]
    fn caches_output_logs_in_tarball() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFile");
        });

        let hash = extract_hash_from_run(sandbox.path(), "outputs:generateFile");
        let tarball = sandbox
            .path()
            .join(".moon/cache/outputs")
            .join(format!("{hash}.tar.gz"));
        let dir = sandbox.path().join(".moon/cache/outputs").join(hash);

        moon_archive::untar(tarball, &dir, None).unwrap();

        assert!(dir.join("stdout.log").exists());
        assert!(dir.join("stderr.log").exists());
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

        assert!(predicate::str::contains("cached from previous run").eval(&assert.output()));

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputs:generateFixed").arg("-u");
        });

        assert!(!predicate::str::contains("cached from previous run").eval(&assert.output()));
    }

    mod hydration {
        use super::*;
        use moon_test_utils::pretty_assertions::assert_eq;

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

        #[test]
        fn hydrates_with_a_different_hash_cache() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash1 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
            let contents1 = fs::read_to_string(sandbox.path().join("outputs/lib/one.js")).unwrap();

            // Create a file to trigger an inputs change
            sandbox.create_file("outputs/trigger.js", "");

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:generateFileAndFolder");
            });

            let hash2 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
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

            let hash3 = extract_hash_from_run(sandbox.path(), "outputs:generateFileAndFolder");
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

    mod archiving {
        use super::*;

        #[test]
        fn doesnt_archive_non_build_tasks() {
            let sandbox = cases_sandbox();
            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            assert!(!sandbox
                .path()
                .join(format!(".moon/cache/outputs/{hash}.tar.gz"))
                .exists());
        }

        #[test]
        fn archives_non_build_tasks_with_full_target() {
            let sandbox = cases_sandbox_with_config(|cfg| {
                cfg.runner
                    .archivable_targets
                    .push("outputs:noOutput".into());
            });

            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            assert!(sandbox
                .path()
                .join(format!(".moon/cache/outputs/{hash}.tar.gz"))
                .exists());
        }

        #[test]
        fn archives_non_build_tasks_with_all_target() {
            let sandbox = cases_sandbox_with_config(|cfg| {
                cfg.runner.archivable_targets.push(":noOutput".into());
            });

            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            assert!(sandbox
                .path()
                .join(format!(".moon/cache/outputs/{hash}.tar.gz"))
                .exists());
        }

        #[test]
        fn doesnt_archive_non_build_tasks_for_nonmatch_target() {
            let sandbox = cases_sandbox_with_config(|cfg| {
                cfg.runner.archivable_targets.push(":otherTarget".into());
            });

            sandbox.enable_git();

            sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            let hash = extract_hash_from_run(sandbox.path(), "outputs:noOutput");

            assert!(!sandbox
                .path()
                .join(format!(".moon/cache/outputs/{hash}.tar.gz"))
                .exists());
        }

        #[test]
        fn archives_std_output() {
            let sandbox = cases_sandbox_with_config(|cfg| {
                cfg.runner.archivable_targets.push(":noOutput".into());
            });

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
            let sandbox = cases_sandbox_with_config(|cfg| {
                cfg.runner.archivable_targets.push(":noOutput".into());
            });

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
        fn errors_for_deps_target() {
            let sandbox = cases_sandbox_with_config(|cfg| {
                cfg.runner.archivable_targets.push("^:otherTarget".into());
            });

            sandbox.enable_git();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            assert!(predicates::str::contains(
                "Project dependencies scope (^:) is not supported in run contexts."
            )
            .eval(&assert.output()));
        }

        #[test]
        fn errors_for_self_target() {
            let sandbox = cases_sandbox_with_config(|cfg| {
                cfg.runner.archivable_targets.push("~:otherTarget".into());
            });

            sandbox.enable_git();

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("run").arg("outputs:noOutput");
            });

            assert!(predicates::str::contains(
                "Project self scope (~:) is not supported in run contexts."
            )
            .eval(&assert.output()));
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

    #[cfg(not(windows))] // Different path output in snapshot
    #[test]
    fn buffer_on_failure_when_failure() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("outputStyles:bufferFailureFailPrimary");
        });

        assert_snapshot!(assert.output());
    }

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

        assert!(predicate::str::contains(
            "Target(s) files:noop not affected by touched files (using status all)"
        )
        .eval(&output));
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

        assert!(predicate::str::contains(
            "Target(s) files:noop not affected by touched files (using status untracked, deleted)"
        )
        .eval(&output));
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

        if cfg!(windows) {
            assert!(predicate::str::contains("\n.\\file.txt\n").eval(&assert.output()));
        } else {
            assert!(predicate::str::contains("\n./file.txt\n").eval(&assert.output()));
        }

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

        if cfg!(windows) {
            assert!(predicate::str::contains("\n.\\other.txt\n").eval(&assert.output()));
        } else {
            assert!(predicate::str::contains("\n./other.txt\n").eval(&assert.output()));
        }

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

        if cfg!(windows) {
            assert!(predicate::str::contains("\n.\\file.txt,.\\other.txt\n").eval(&assert.output()));
        } else {
            assert!(predicate::str::contains("\n./file.txt,./other.txt\n").eval(&assert.output()));
        }
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

        assert!(predicate::str::contains(
            "Target(s) files:noop not affected by touched files (using status deleted)"
        )
        .eval(&output));
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

            assert!(
                predicate::str::contains("Target(s) root:noop not affected by touched files")
                    .eval(&output)
            );
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

            assert!(predicate::str::contains(
                "Target(s) root:noop not affected by touched files (using status deleted)"
            )
            .eval(&output));
        }
    }
}

mod interactive {
    use super::*;

    #[test]
    fn errors_if_more_than_1_target() {
        let sandbox = cases_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg(":noop").arg("--interactive");
        });

        assert.failure().stderr(predicate::str::contains(
            "Only 1 target can be ran as interactive. Requires a fully qualified project target.",
        ));
    }

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
                .write_stdin("with-local");
        });

        // Test doesn't output the input (answer) we provide, so check for the question
        assert
            .success()
            .stdout(predicate::str::contains("Question?"));
    }
}
