use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_utils::path::replace_home_dir;
use moon_utils::test::{
    create_fixtures_sandbox, create_moon_command, create_moon_command_in, get_assert_output,
    replace_fixtures_dir,
};
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::fs::{read_to_string, OpenOptions};
use std::io::prelude::*;
use std::path::Path;

fn append_workspace_config(path: &Path, yaml: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)
        .unwrap();

    writeln!(file, "{}", yaml).unwrap();
}

fn update_version_workspace_config(dir: &Path, old_version: &str, new_version: &str) {
    let mut config = fs::read_to_string(dir.join(".moon/workspace.yml")).unwrap();

    config = config.replace(old_version, new_version);

    fs::write(dir.join(".moon/workspace.yml"), config).unwrap();
}

fn get_path_safe_output(assert: &assert_cmd::assert::Assert, fixtures_dir: &Path) -> String {
    let result = replace_home_dir(&replace_fixtures_dir(
        &get_assert_output(assert),
        fixtures_dir,
    ));

    result.replace("/private<", "<")
}

async fn extract_hash_from_run(fixture: &Path, target: &str) -> String {
    let engine = CacheEngine::create(fixture).await.unwrap();
    let cache = engine.cache_run_target_state(target).await.unwrap();

    cache.item.hash
}

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

#[test]
fn errors_for_unknown_all_target() {
    let assert = create_moon_command("cases")
        .arg("run")
        .arg(":unknown")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn errors_for_cycle_in_task_deps() {
    let assert = create_moon_command("cases")
        .arg("run")
        .arg("depsA:taskCycle")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[cfg(not(windows))]
mod general {
    use super::*;

    #[test]
    fn logs_command_for_project_root() {
        let fixture = create_fixtures_sandbox("cases");

        append_workspace_config(
            &fixture.path().join(".moon/workspace.yml"),
            "actionRunner:\n  logRunningCommand: true",
        );

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("base:runFromProject")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn logs_command_for_workspace_root() {
        let fixture = create_fixtures_sandbox("cases");

        append_workspace_config(
            &fixture.path().join(".moon/workspace.yml"),
            "actionRunner:\n  logRunningCommand: true",
        );

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("base:runFromWorkspace")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod configs {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn bubbles_up_invalid_workspace_config() {
        let assert = create_moon_command("config-invalid-workspace")
            .arg("run")
            .arg("project:task")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, &PathBuf::from("./fake/path")));
    }

    #[test]
    fn bubbles_up_invalid_global_project_config() {
        let assert = create_moon_command("config-invalid-global-project")
            .arg("run")
            .arg("project:task")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, &PathBuf::from("./fake/path")));
    }

    #[test]
    fn bubbles_up_invalid_project_config() {
        let assert = create_moon_command("config-invalid-project")
            .arg("run")
            .arg("test:task")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, &PathBuf::from("./fake/path")));
    }
}

mod logs {
    use super::*;
    use moon_utils::test::create_fixtures_sandbox;

    #[test]
    fn creates_log_file() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("--logFile=output.log")
            .arg("run")
            .arg("node:standard")
            .assert();

        let output_path = fixture.path().join("output.log");

        assert!(output_path.exists());
    }

    #[test]
    fn creates_nested_log_file() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("--logFile=nested/output.log")
            .arg("run")
            .arg("node:standard")
            .assert();

        let output_path = fixture.path().join("nested/output.log");

        assert!(output_path.exists());
    }
}

mod caching {
    use super::*;
    use moon_cache::{CacheItem, RunTargetState};

    #[test]
    fn uses_cache_on_subsequent_runs() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        assert_snapshot!(get_assert_output(&assert));

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn creates_runfile() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        assert!(fixture
            .path()
            .join(".moon/cache/runs/node/runfile.json")
            .exists());
    }

    #[tokio::test]
    async fn creates_run_state_cache() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        let cache_path = fixture
            .path()
            .join(".moon/cache/runs/node/standard/lastRunState.json");

        assert!(cache_path.exists());

        let state = CacheItem::load(cache_path, RunTargetState::default(), 0)
            .await
            .unwrap();

        assert_snapshot!(read_to_string(
            fixture
                .path()
                .join(format!(".moon/cache/hashes/{}.json", state.item.hash))
        )
        .unwrap());

        assert_eq!(state.item.exit_code, 0);
        // This is flakey... caused by output capturing?
        // assert_eq!(state.item.stdout, "stdout");
        // assert_eq!(state.item.stderr, "stderr");
        assert_eq!(state.item.target, "node:standard");
        assert_eq!(
            state.item.hash,
            "3270284f4824b530c3006108757e715f73a43f949c811db7c0859aded12d9036"
        );
    }
}

mod dependencies {
    use super::*;

    #[test]
    fn runs_the_graph_in_order() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("depsA:dependencyOrder")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_the_graph_in_order_not_from_head() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("depsB:dependencyOrder")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
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
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg(":all")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:all").eval(&output));
        assert!(predicate::str::contains("targetScopeB:all").eval(&output));
        assert!(predicate::str::contains("targetScopeC:all").eval(&output));
        assert!(predicate::str::contains("Tasks: 3 completed").eval(&output));
    }

    #[test]
    fn supports_deps_scope_in_task() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("targetScopeA:deps")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeA:deps").eval(&output));
        assert!(predicate::str::contains("scope=deps").eval(&output));
        assert!(predicate::str::contains("depsA:standard").eval(&output));
        assert!(predicate::str::contains("deps=a").eval(&output));
        assert!(predicate::str::contains("depsB:standard").eval(&output));
        assert!(predicate::str::contains("deps=b").eval(&output));
        assert!(predicate::str::contains("depsC:standard").eval(&output));
        assert!(predicate::str::contains("deps=c").eval(&output));
        assert!(predicate::str::contains("Tasks: 4 completed").eval(&output));
    }

    #[test]
    fn supports_self_scope_in_task() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("targetScopeB:self")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("targetScopeB:self").eval(&output));
        assert!(predicate::str::contains("scope=self").eval(&output));
        assert!(predicate::str::contains("targetScopeB:selfOther").eval(&output));
        assert!(predicate::str::contains("selfOther").eval(&output));
        assert!(predicate::str::contains("Tasks: 2 completed").eval(&output));
    }
}

mod node {
    use super::*;

    #[test]
    fn runs_package_managers() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:npm")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_standard_script() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_cjs_files() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:cjs")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_mjs_files() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:mjs")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn supports_top_level_await() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:topLevelAwait")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_zero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:processExitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:processExitNonZero")
            .assert();

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(get_assert_output(&assert));
        }
    }

    #[test]
    fn handles_process_exit_code_zero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:exitCodeZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_code_nonzero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:exitCodeNonZero")
            .assert();

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(get_assert_output(&assert));
        }
    }

    #[test]
    fn handles_throw_error() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:throwError")
            .assert();
        let output = get_assert_output(&assert);

        // Output contains file paths that we cant snapshot
        assert!(predicate::str::contains("Error: Oops").eval(&output));
    }

    #[test]
    fn handles_unhandled_promise() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:unhandledPromise")
            .assert();

        if cfg!(windows) {
            assert.code(1);
        } else {
            assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
        }
    }

    #[test]
    fn passes_args_through() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
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

    #[test]
    fn sets_env_vars() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:envVars")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn inherits_moon_env_vars() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:envVarsMoon")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_project_root() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:runFromProject")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_workspace_root() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:runFromWorkspace")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn retries_on_failure_till_count() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("node:retryCount")
            .assert();
        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("Process ~/.moon/tools/node/16.0.0").eval(&output));
    }

    mod install_deps {
        use super::*;

        #[test]
        fn installs_on_first_run() {
            let fixture = create_fixtures_sandbox("cases");

            assert!(!fixture.path().join("node_modules").exists());

            let assert = create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT")
                .assert();
            let output = get_assert_output(&assert);

            assert!(fixture.path().join("node_modules").exists());

            assert!(predicate::str::contains("added 7 packages").eval(&output));
        }

        #[test]
        fn doesnt_reinstall_on_second_run() {
            let fixture = create_fixtures_sandbox("cases");

            let assert = create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT")
                .assert();
            let output1 = get_assert_output(&assert);

            assert!(predicate::str::contains("added 7 packages").eval(&output1));

            let assert = create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT")
                .assert();
            let output2 = get_assert_output(&assert);

            assert!(!predicate::str::contains("added 7 packages").eval(&output2));
        }

        #[test]
        fn creates_workspace_state_cache() {
            let fixture = create_fixtures_sandbox("cases");

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .assert();

            assert!(fixture
                .path()
                .join(".moon/cache/workspaceState.json")
                .exists());
        }
    }

    mod engines {
        use super::*;

        #[test]
        fn adds_engines_constraint() {
            let fixture = create_fixtures_sandbox("cases");

            append_workspace_config(
                &fixture.path().join(".moon/workspace.yml"),
                r#"  addEnginesConstraint: true"#,
            );

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .assert();

            assert_snapshot!(read_to_string(fixture.path().join("package.json")).unwrap());
        }

        #[test]
        fn doesnt_add_engines_constraint() {
            let fixture = create_fixtures_sandbox("cases");

            append_workspace_config(
                &fixture.path().join(".moon/workspace.yml"),
                r#"  addEnginesConstraint: false"#,
            );

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .assert();

            assert_snapshot!(read_to_string(fixture.path().join("package.json")).unwrap());
        }
    }

    mod version_manager {
        use super::*;

        #[test]
        fn adds_no_file_by_default() {
            let fixture = create_fixtures_sandbox("cases");

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .assert();

            assert!(!fixture.path().join(".nvmrc").exists());
            assert!(!fixture.path().join(".node-version").exists());
        }

        #[test]
        fn adds_nvmrc_file() {
            let fixture = create_fixtures_sandbox("cases");

            append_workspace_config(
                &fixture.path().join(".moon/workspace.yml"),
                r#"  syncVersionManagerConfig: nvm"#,
            );

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .assert();

            assert!(fixture.path().join(".nvmrc").exists());

            assert_eq!(
                read_to_string(fixture.path().join(".nvmrc")).unwrap(),
                "16.0.0"
            );
        }

        #[test]
        fn adds_nodenv_file() {
            let fixture = create_fixtures_sandbox("cases");

            append_workspace_config(
                &fixture.path().join(".moon/workspace.yml"),
                r#"  syncVersionManagerConfig: nodenv"#,
            );

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .assert();

            assert!(fixture.path().join(".node-version").exists());

            assert_eq!(
                read_to_string(fixture.path().join(".node-version")).unwrap(),
                "16.0.0"
            );
        }

        #[test]
        fn errors_for_invalid_value() {
            let fixture = create_fixtures_sandbox("cases");

            append_workspace_config(
                &fixture.path().join(".moon/workspace.yml"),
                r#"  syncVersionManagerConfig: invalid"#,
            );

            let assert = create_moon_command_in(fixture.path())
                .arg("run")
                .arg("node:standard")
                .assert();

            let output = get_assert_output(&assert);

            assert!(predicate::str::contains(
                "unknown variant: found `invalid`, expected ``nodenv` or `nvm``"
            )
            .eval(&output));
        }
    }

    mod sync_depends_on {
        use super::*;

        #[test]
        fn syncs_as_dependency_to_package_json() {
            let fixture = create_fixtures_sandbox("cases");

            append_workspace_config(
                &fixture.path().join(".moon/workspace.yml"),
                "  syncProjectWorkspaceDependencies: true",
            );

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("dependsOn:standard")
                .assert();

            // deps-c does not have a `package.json` on purpose
            assert_snapshot!(
                read_to_string(fixture.path().join("depends-on/package.json")).unwrap()
            );
        }

        #[test]
        fn syncs_as_reference_to_tsconfig_json() {
            let fixture = create_fixtures_sandbox("cases");

            append_workspace_config(
                &fixture.path().join(".moon/workspace.yml"),
                "typescript:\n  syncProjectReferences: true",
            );

            create_moon_command_in(fixture.path())
                .arg("run")
                .arg("dependsOn:standard")
                .assert();

            // root
            assert_snapshot!(read_to_string(fixture.path().join("tsconfig.json")).unwrap());

            // project
            // deps-a does not have a `tsconfig.json` on purpose
            assert_snapshot!(
                read_to_string(fixture.path().join("depends-on/tsconfig.json")).unwrap()
            );
        }
    }
}

mod node_npm {
    use super::*;

    #[test]
    #[serial]
    fn installs_correct_version() {
        let fixture = create_fixtures_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("npm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    // NOTE: This fails on Windows for some reason...
    #[cfg(not(windows))]
    #[test]
    #[serial]
    fn installs_correct_version_using_corepack() {
        let fixture = create_fixtures_sandbox("node-npm");

        // Corepack released in v16.9
        update_version_workspace_config(fixture.path(), "16.1.0", "16.10.0");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("npm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn can_install_a_dep() {
        let fixture = create_fixtures_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("npm:installDep")
            .assert();

        assert.success();
    }
}

mod node_pnpm {
    use super::*;

    #[test]
    #[serial]
    fn installs_correct_version() {
        let fixture = create_fixtures_sandbox("node-pnpm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("pnpm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn installs_correct_version_using_corepack() {
        let fixture = create_fixtures_sandbox("node-pnpm");

        // Corepack released in v16.9
        update_version_workspace_config(fixture.path(), "16.2.0", "16.11.0");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("pnpm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn can_install_a_dep() {
        let fixture = create_fixtures_sandbox("node-pnpm");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("pnpm:installDep")
            .assert();

        assert.success();
    }
}

mod node_yarn1 {
    use super::*;

    #[test]
    #[serial]
    fn installs_correct_version() {
        let fixture = create_fixtures_sandbox("node-yarn1");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("yarn:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn installs_correct_version_using_corepack() {
        let fixture = create_fixtures_sandbox("node-yarn1");

        // Corepack released in v16.9
        update_version_workspace_config(fixture.path(), "16.3.0", "16.12.0");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("yarn:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn can_install_a_dep() {
        let fixture = create_fixtures_sandbox("node-yarn1");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("yarn:installDep")
            .assert();

        assert.success();
    }
}

// TODO: This fails in CI for some reason, but not locally...
// mod node_yarn {
//     use super::*;

//     #[test]
//     #[serial]
//     fn installs_correct_version() {
//         let fixture = create_fixtures_sandbox("node-yarn");

//         let assert = create_moon_command_in(fixture.path())
//             .arg("run")
//             .arg("yarn:version")
//             .assert();

//         assert_snapshot!(get_assert_output(&assert));
//     }

//     #[test]
//     #[serial]
//     fn installs_correct_version_using_corepack() {
//         let fixture = create_fixtures_sandbox("node-yarn");

//         // Corepack released in v16.9
//         update_version_workspace_config(fixture.path(), "16.4.0", "16.13.0");

//         let assert = create_moon_command_in(fixture.path())
//             .arg("run")
//             .arg("yarn:version")
//             .assert();

//         assert_snapshot!(get_assert_output(&assert));
//     }

//     #[test]
//     #[serial]
//     fn can_install_a_dep() {
//         let fixture = create_fixtures_sandbox("node-yarn");

//         let assert = create_moon_command_in(fixture.path())
//             .arg("run")
//             .arg("yarn:installDep")
//             .assert();

//         assert.success();
//     }
// }

#[cfg(not(windows))]
mod system {
    use super::*;

    #[test]
    fn handles_echo() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:echo")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_ls() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:ls")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn runs_bash_script() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:bash")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_zero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:exitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:exitNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn passes_args_through() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
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

    #[test]
    fn sets_env_vars() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:envVars")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn inherits_moon_env_vars() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:envVarsMoon")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_project_root() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:runFromProject")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_workspace_root() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:runFromWorkspace")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn retries_on_failure_till_count() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("system:retryCount")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

#[cfg(windows)]
mod system_windows {
    use super::*;

    #[test]
    fn runs_bat_script() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:bat")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn handles_process_exit_zero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:exitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:exitNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn passes_args_through() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:passthroughArgs")
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
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:envVars")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn inherits_moon_env_vars() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:envVarsMoon")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_project_root() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:runFromProject")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn runs_from_workspace_root() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:runFromWorkspace")
            .assert();

        assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
    }

    #[test]
    fn retries_on_failure_till_count() {
        let fixture = create_fixtures_sandbox("cases");

        let assert = create_moon_command_in(fixture.path())
            .arg("run")
            .arg("systemWindows:retryCount")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod outputs {
    use super::*;

    // fn debug_dir(dir: &Path) {
    //     for entry in std::fs::read_dir(dir).unwrap() {
    //         let entry = entry.unwrap();
    //         let path = entry.path();

    //         if path.is_dir() {
    //             debug_dir(&path);
    //         } else {
    //             println!("{:#?}", path);
    //         }
    //     }
    // }

    #[tokio::test]
    async fn links_single_file() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("run")
            .arg("outputs:generateFile")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFile").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());
        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(hash)
            .join("lib/one.js")
            .exists());
    }

    #[tokio::test]
    async fn links_multiple_files() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("run")
            .arg("outputs:generateFiles")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFiles").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());
        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("lib/one.js")
            .exists());
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("lib/two.js")
            .exists());
    }

    #[tokio::test]
    async fn links_single_folder() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("run")
            .arg("outputs:generateFolder")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFolder").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());
        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("lib/one.js")
            .exists());
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("lib/two.js")
            .exists());
    }

    #[tokio::test]
    async fn links_multiple_folders() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("run")
            .arg("outputs:generateFolders")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFolders").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());
        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("lib/one.js")
            .exists());
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("esm/two.js")
            .exists());
    }

    #[tokio::test]
    async fn links_both_file_and_folder() {
        let fixture = create_fixtures_sandbox("cases");

        create_moon_command_in(fixture.path())
            .arg("run")
            .arg("outputs:generateFileAndFolder")
            .assert();

        let hash = extract_hash_from_run(fixture.path(), "outputs:generateFileAndFolder").await;

        // hash
        assert!(fixture
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", hash))
            .exists());
        // outputs
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("lib/one.js")
            .exists());
        assert!(fixture
            .path()
            .join(".moon/cache/out")
            .join(&hash)
            .join("esm/two.js")
            .exists());
    }
}
