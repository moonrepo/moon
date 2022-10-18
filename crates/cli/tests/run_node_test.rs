mod utils;

use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox_with_git, get_assert_output};
use predicates::prelude::*;
use serial_test::serial;
use std::fs::read_to_string;
use utils::{append_workspace_config, get_path_safe_output, update_workspace_config};

#[test]
fn runs_package_managers() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:npm")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn runs_standard_script() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:standard")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn runs_cjs_files() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:cjs")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn runs_mjs_files() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:mjs")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn supports_top_level_await() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:topLevelAwait")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn handles_process_exit_zero() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:processExitZero")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn handles_process_exit_nonzero() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
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
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:exitCodeZero")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn handles_process_exit_code_nonzero() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
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
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:throwError")
        .assert();
    let output = get_assert_output(&assert);

    // Output contains file paths that we cant snapshot
    assert!(predicate::str::contains("Error: Oops").eval(&output));
}

#[test]
fn handles_unhandled_promise() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
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
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
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
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:envVars")
        .assert();

    assert_snapshot!(get_assert_output(&assert));
}

#[test]
fn inherits_moon_env_vars() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:envVarsMoon")
        .assert();

    assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
}

#[test]
fn runs_from_project_root() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:runFromProject")
        .assert();

    assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
}

#[test]
fn runs_from_workspace_root() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:runFromWorkspace")
        .assert();

    assert_snapshot!(get_path_safe_output(&assert, fixture.path()));
}

#[test]
fn retries_on_failure_till_count() {
    let fixture = create_sandbox_with_git("cases");

    let assert = create_moon_command(fixture.path())
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
        let fixture = create_sandbox_with_git("cases");

        assert!(!fixture.path().join("node_modules").exists());

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("node:standard")
            .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT")
            .assert();
        let output = get_assert_output(&assert);

        assert!(fixture.path().join("node_modules").exists());

        assert!(predicate::str::contains("added 9 packages").eval(&output));
    }

    #[test]
    fn doesnt_reinstall_on_second_run() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("node:standard")
            .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT")
            .assert();
        let output1 = get_assert_output(&assert);

        assert!(predicate::str::contains("added 9 packages").eval(&output1));

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("node:standard")
            .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT")
            .assert();
        let output2 = get_assert_output(&assert);

        assert!(!predicate::str::contains("added 9 packages").eval(&output2));
    }

    #[test]
    fn creates_tool_state_cache() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        assert!(fixture
            .path()
            .join(".moon/cache/states/toolNode-16.0.0.json")
            .exists());
    }

    #[test]
    fn installs_deps_into_each_project_when_not_using_workspaces() {
        let fixture = create_sandbox_with_git("node-non-workspaces");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("foo:noop")
            .arg("bar:noop")
            .arg("baz:noop")
            .assert();

        assert!(predicate::str::contains("npm install")
            .count(3)
            .eval(&get_assert_output(&assert)));

        assert!(fixture.path().join("foo/package-lock.json").exists());
        assert!(fixture.path().join("bar/package-lock.json").exists());
        assert!(fixture.path().join("baz/package-lock.json").exists());
    }
}

mod engines {
    use super::*;

    #[test]
    fn adds_engines_constraint() {
        let fixture = create_sandbox_with_git("cases");

        update_workspace_config(
            fixture.path(),
            "addEnginesConstraint: false",
            "addEnginesConstraint: true",
        );

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        moon_utils::test::debug_sandbox(&fixture, &assert);

        assert_snapshot!(read_to_string(fixture.path().join("package.json")).unwrap());
    }

    #[test]
    fn doesnt_add_engines_constraint() {
        let fixture = create_sandbox_with_git("cases");

        append_workspace_config(fixture.path(), r#"  addEnginesConstraint: false"#);

        create_moon_command(fixture.path())
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
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("node:standard")
            .assert();

        assert!(!fixture.path().join(".nvmrc").exists());
        assert!(!fixture.path().join(".node-version").exists());
    }

    #[test]
    fn adds_nvmrc_file() {
        let fixture = create_sandbox_with_git("cases");

        append_workspace_config(fixture.path(), r#"  syncVersionManagerConfig: nvm"#);

        create_moon_command(fixture.path())
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
        let fixture = create_sandbox_with_git("cases");

        append_workspace_config(fixture.path(), r#"  syncVersionManagerConfig: nodenv"#);

        create_moon_command(fixture.path())
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
        let fixture = create_sandbox_with_git("cases");

        append_workspace_config(fixture.path(), r#"  syncVersionManagerConfig: invalid"#);

        let assert = create_moon_command(fixture.path())
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

    fn test_depends_on_format(format: &str) {
        let fixture = create_sandbox_with_git("cases");

        update_workspace_config(
            fixture.path(),
            "syncProjectWorkspaceDependencies: false",
            "syncProjectWorkspaceDependencies: true",
        );

        append_workspace_config(
            fixture.path(),
            &format!("  dependencyVersionFormat: {}", format),
        );

        create_moon_command(fixture.path())
            .arg("run")
            .arg("dependsOn:standard")
            .assert();

        // deps-c does not have a `package.json` on purpose
        assert_snapshot!(
            format!("format_{}", format),
            read_to_string(fixture.path().join("depends-on/package.json")).unwrap()
        );
    }

    #[test]
    fn syncs_as_file_dependency() {
        test_depends_on_format("file");
    }

    #[test]
    fn syncs_as_link_dependency() {
        test_depends_on_format("link");
    }

    #[test]
    fn syncs_as_star_dependency() {
        test_depends_on_format("star");
    }

    #[test]
    fn syncs_as_version_dependency() {
        test_depends_on_format("version");
    }

    #[test]
    fn syncs_as_version_caret_dependency() {
        test_depends_on_format("version-caret");
    }

    #[test]
    fn syncs_as_version_tilde_dependency() {
        test_depends_on_format("version-tilde");
    }

    #[test]
    fn syncs_as_workspace_dependency() {
        test_depends_on_format("workspace");
    }

    #[test]
    fn syncs_as_workspace_caret_dependency() {
        test_depends_on_format("workspace-caret");
    }

    #[test]
    fn syncs_as_workspace_tilde_dependency() {
        test_depends_on_format("workspace-tilde");
    }

    #[test]
    fn syncs_depends_on_with_scopes() {
        let fixture = create_sandbox_with_git("cases");

        update_workspace_config(
            fixture.path(),
            "syncProjectWorkspaceDependencies: false",
            "syncProjectWorkspaceDependencies: true",
        );

        create_moon_command(fixture.path())
            .arg("run")
            .arg("dependsOnScopes:standard")
            .assert();

        // deps-c does not have a `package.json` on purpose
        assert_snapshot!(
            read_to_string(fixture.path().join("depends-on-scopes/package.json")).unwrap()
        );
    }
}

mod npm {
    use super::*;

    #[test]
    #[serial]
    fn installs_correct_version() {
        let fixture = create_sandbox_with_git("node-npm");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("npm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn installs_correct_version_using_corepack() {
        let fixture = create_sandbox_with_git("node-npm");

        // Corepack released in v16.9
        update_workspace_config(fixture.path(), "16.1.0", "16.10.0");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("npm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn can_install_a_dep() {
        let fixture = create_sandbox_with_git("node-npm");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("npm:installDep")
            .assert();

        assert.success();
    }

    #[test]
    #[serial]
    fn can_run_a_deps_bin() {
        let fixture = create_sandbox_with_git("node-npm");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("npm:runDep")
            .assert();

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&get_assert_output(&assert))
        );

        assert.success();
    }

    #[test]
    #[serial]
    fn installs_deps_in_non_workspace_project() {
        let fixture = create_sandbox_with_git("node-npm");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("notInWorkspace:noop")
            // Run other package so we can see both working
            .arg("npm:noop")
            .assert();

        assert!(predicate::str::contains("npm install")
            .count(2)
            .eval(&get_assert_output(&assert)));

        assert!(fixture.path().join("package-lock.json").exists());
        assert!(fixture
            .path()
            .join("not-in-workspace/package-lock.json")
            .exists());
        assert!(fixture
            .path()
            .join("not-in-workspace/node_modules")
            .exists());

        assert.success();
    }
}

mod pnpm {
    use super::*;
    use std::fs;

    #[test]
    #[serial]
    fn installs_correct_version() {
        let fixture = create_sandbox_with_git("node-pnpm");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("pnpm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn installs_correct_version_using_corepack() {
        let fixture = create_sandbox_with_git("node-pnpm");

        // Corepack released in v16.9
        update_workspace_config(fixture.path(), "16.2.0", "16.11.0");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("pnpm:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn can_install_a_dep() {
        let fixture = create_sandbox_with_git("node-pnpm");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("pnpm:installDep")
            .assert();

        assert.success();
    }

    #[test]
    #[serial]
    fn can_run_a_deps_bin_isolated() {
        let fixture = create_sandbox_with_git("node-pnpm");

        fs::write(fixture.path().join(".npmrc"), "node-linker=isolated").unwrap();

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("pnpm:runDep")
            .assert();

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&get_assert_output(&assert))
        );

        assert.success();
    }

    #[test]
    #[serial]
    fn can_run_a_deps_bin_hoisted() {
        let fixture = create_sandbox_with_git("node-pnpm");

        fs::write(fixture.path().join(".npmrc"), "node-linker=hoisted").unwrap();

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("pnpm:runDep")
            .assert();

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&get_assert_output(&assert))
        );

        assert.success();
    }

    // NOTE: pnpm does not support nested lockfiles.
    // #[test]
    // #[serial]
    // fn installs_deps_in_non_workspace_project() {
    //     let fixture = create_sandbox_with_git("node-pnpm");

    //     let assert = create_moon_command(fixture.path())
    //         .arg("run")
    //         .arg("notInWorkspace:noop")
    //         // Run other package so we can see both working
    //         .arg("pnpm:noop")
    //         .assert();

    //     assert_snapshot!(get_assert_output(&assert));

    //     assert!(fixture.path().join("pnpm-lock.yaml").exists());
    //     assert!(fixture
    //         .path()
    //         .join("not-in-workspace/pnpm-lock.yaml")
    //         .exists());
    //     assert!(fixture
    //         .path()
    //         .join("not-in-workspace/node_modules")
    //         .exists());

    //     assert.success();
    // }
}

mod yarn1 {
    use super::*;

    #[test]
    #[serial]
    fn installs_correct_version() {
        let fixture = create_sandbox_with_git("node-yarn1");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn installs_correct_version_using_corepack() {
        let fixture = create_sandbox_with_git("node-yarn1");

        // Corepack released in v16.9
        update_workspace_config(fixture.path(), "16.3.0", "16.12.0");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn can_install_a_dep() {
        let fixture = create_sandbox_with_git("node-yarn1");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:installDep")
            .assert();

        assert.success();
    }

    #[test]
    #[serial]
    fn can_run_a_deps_bin() {
        let fixture = create_sandbox_with_git("node-yarn1");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:runDep")
            .assert();

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&get_assert_output(&assert))
        );

        assert.success();
    }

    #[test]
    #[serial]
    fn installs_deps_in_non_workspace_project() {
        let fixture = create_sandbox_with_git("node-yarn1");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("notInWorkspace:noop")
            // Run other package so we can see both working
            .arg("yarn:noop")
            .assert();

        assert!(predicate::str::contains("yarn install")
            .count(2)
            .eval(&get_assert_output(&assert)));

        assert!(fixture.path().join("yarn.lock").exists());
        assert!(fixture.path().join("not-in-workspace/yarn.lock").exists());
        assert!(fixture
            .path()
            .join("not-in-workspace/node_modules")
            .exists());

        assert.success();
    }
}

mod yarn {
    use super::*;

    #[test]
    #[serial]
    fn installs_correct_version() {
        let fixture = create_sandbox_with_git("node-yarn");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn installs_correct_version_using_corepack() {
        let fixture = create_sandbox_with_git("node-yarn");

        // Corepack released in v16.9
        update_workspace_config(fixture.path(), "16.4.0", "16.13.0");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:version")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    #[serial]
    fn can_install_a_dep() {
        let fixture = create_sandbox_with_git("node-yarn");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:installDep")
            .assert();

        assert.success();
    }

    #[test]
    #[serial]
    fn can_run_a_deps_bin() {
        let fixture = create_sandbox_with_git("node-yarn");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("yarn:runDep")
            .assert();

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&get_assert_output(&assert))
        );

        assert.success();
    }

    #[test]
    #[serial]
    fn installs_deps_in_non_workspace_project() {
        let fixture = create_sandbox_with_git("node-yarn");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("notInWorkspace:noop")
            // Run other package so we can see both working
            .arg("yarn:noop")
            .assert();

        assert!(predicate::str::contains("yarn install")
            .count(2)
            .eval(&get_assert_output(&assert)));

        assert!(fixture.path().join("yarn.lock").exists());
        assert!(fixture.path().join("not-in-workspace/yarn.lock").exists());
        assert!(fixture
            .path()
            .join("not-in-workspace/node_modules")
            .exists());

        assert.success();
    }
}

mod profile {
    use super::*;

    #[test]
    fn record_a_cpu_profile() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("--profile")
            .arg("cpu")
            .arg("node:standard")
            .assert();

        let profile = fixture
            .path()
            .join(".moon/cache/states/node/standard/snapshot.cpuprofile");

        assert!(profile.exists());
    }

    #[test]
    fn record_a_heap_profile() {
        let fixture = create_sandbox_with_git("cases");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("--profile")
            .arg("heap")
            .arg("node:standard")
            .assert();

        let profile = fixture
            .path()
            .join(".moon/cache/states/node/standard/snapshot.heapprofile");

        assert!(profile.exists());
    }
}

mod aliases {
    use super::*;

    #[test]
    fn runs_via_package_name() {
        let fixture = create_sandbox_with_git("project-graph/aliases");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("@scope/pkg-foo:standard")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}

mod non_js_bins {
    use super::*;
    use std::fs;

    #[test]
    fn works_with_esbuild() {
        let fixture = create_sandbox_with_git("node");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("esbuild:build")
            .assert()
            .success();

        assert_eq!(
            fs::read_to_string(fixture.path().join("esbuild/out.js")).unwrap(),
            "(() => {\n  // index.js\n  var ESBUILD = \"esbuild\";\n})();\n"
        );
    }

    #[test]
    fn works_with_swc() {
        let fixture = create_sandbox_with_git("node");

        create_moon_command(fixture.path())
            .arg("run")
            .arg("swc:build")
            .assert()
            .success();

        assert_eq!(
            fs::read_to_string(fixture.path().join("swc/out.js")).unwrap(),
            "export var SWC = \"swc\";\n\n\n//# sourceMappingURL=out.js.map"
        );
    }
}

mod typescript {
    use super::*;

    #[test]
    fn creates_missing_tsconfig() {
        let fixture = create_sandbox_with_git("typescript");

        assert!(!fixture.path().join("create-config/tsconfig.json").exists());

        create_moon_command(fixture.path())
            .arg("run")
            .arg("create-config:test")
            .assert();

        assert!(fixture.path().join("create-config/tsconfig.json").exists());

        // root
        assert_snapshot!(read_to_string(fixture.path().join("tsconfig.json")).unwrap());

        // project
        assert_snapshot!(
            read_to_string(fixture.path().join("create-config/tsconfig.json")).unwrap()
        );
    }

    #[test]
    fn doesnt_create_missing_tsconfig_if_setting_off() {
        let fixture = create_sandbox_with_git("typescript");

        update_workspace_config(
            fixture.path(),
            "createMissingConfig: true",
            "createMissingConfig: false",
        );

        assert!(!fixture.path().join("create-config/tsconfig.json").exists());

        create_moon_command(fixture.path())
            .arg("run")
            .arg("create-config:test")
            .assert();

        assert!(!fixture.path().join("create-config/tsconfig.json").exists());
    }

    #[test]
    fn doesnt_create_missing_tsconfig_if_syncing_off() {
        let fixture = create_sandbox_with_git("typescript");

        update_workspace_config(
            fixture.path(),
            "syncProjectReferences: true",
            "syncProjectReferences: false",
        );

        assert!(!fixture.path().join("create-config/tsconfig.json").exists());

        create_moon_command(fixture.path())
            .arg("run")
            .arg("create-config:test")
            .assert();

        assert!(!fixture.path().join("create-config/tsconfig.json").exists());
    }

    #[test]
    fn doesnt_create_missing_tsconfig_if_project_disabled() {
        let fixture = create_sandbox_with_git("typescript");

        assert!(!fixture.path().join("create-config/tsconfig.json").exists());

        create_moon_command(fixture.path())
            .arg("run")
            .arg("create-config-disabled:test")
            .assert();

        assert!(!fixture.path().join("create-config/tsconfig.json").exists());
    }

    #[test]
    fn syncs_ref_to_root_config() {
        let fixture = create_sandbox_with_git("typescript");

        let initial_root = read_to_string(fixture.path().join("tsconfig.json")).unwrap();

        create_moon_command(fixture.path())
            .arg("run")
            .arg("create-config:test")
            .assert();

        let synced_root = read_to_string(fixture.path().join("tsconfig.json")).unwrap();

        assert_ne!(initial_root, synced_root);
        assert_snapshot!(synced_root);
    }

    #[test]
    fn syncs_depends_on_as_refs() {
        let fixture = create_sandbox_with_git("typescript");

        assert!(!fixture
            .path()
            .join("syncs-deps-refs/tsconfig.json")
            .exists());

        create_moon_command(fixture.path())
            .arg("run")
            .arg("syncs-deps-refs:test")
            .assert();

        // should not have `deps-no-config-disabled` or `deps-with-config-disabled`
        assert_snapshot!(
            read_to_string(fixture.path().join("syncs-deps-refs/tsconfig.json")).unwrap()
        );
    }

    mod out_dir {
        use super::*;

        #[test]
        fn routes_to_cache() {
            let fixture = create_sandbox_with_git("typescript");

            update_workspace_config(
                fixture.path(),
                "routeOutDirToCache: false",
                "routeOutDirToCache: true",
            );

            create_moon_command(fixture.path())
                .arg("run")
                .arg("out-dir-routing:test")
                .assert();

            assert_snapshot!(
                read_to_string(fixture.path().join("out-dir-routing/tsconfig.json")).unwrap()
            );
        }

        #[test]
        fn routes_to_cache_when_no_compiler_options() {
            let fixture = create_sandbox_with_git("typescript");

            update_workspace_config(
                fixture.path(),
                "routeOutDirToCache: false",
                "routeOutDirToCache: true",
            );

            create_moon_command(fixture.path())
                .arg("run")
                .arg("out-dir-routing-no-options:test")
                .assert();

            assert_snapshot!(read_to_string(
                fixture
                    .path()
                    .join("out-dir-routing-no-options/tsconfig.json")
            )
            .unwrap());
        }

        #[test]
        fn doesnt_route_to_cache_if_disabled() {
            let fixture = create_sandbox_with_git("typescript");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("out-dir-routing:test")
                .assert();

            assert_snapshot!(
                read_to_string(fixture.path().join("out-dir-routing/tsconfig.json")).unwrap()
            );
        }
    }

    mod paths {
        use super::*;

        #[test]
        fn maps_paths() {
            let fixture = create_sandbox_with_git("typescript");

            update_workspace_config(
                fixture.path(),
                "syncProjectReferencesToPaths: false",
                "syncProjectReferencesToPaths: true",
            );

            create_moon_command(fixture.path())
                .arg("run")
                .arg("syncs-paths-refs:test")
                .assert();

            assert_snapshot!(
                read_to_string(fixture.path().join("syncs-paths-refs/tsconfig.json")).unwrap()
            );
        }

        #[test]
        fn doesnt_map_paths_if_no_refs() {
            let fixture = create_sandbox_with_git("typescript");

            update_workspace_config(
                fixture.path(),
                "syncProjectReferences: true",
                "syncProjectReferences: false",
            );

            update_workspace_config(
                fixture.path(),
                "syncProjectReferencesToPaths: false",
                "syncProjectReferencesToPaths: true",
            );

            std::fs::remove_file(fixture.path().join("syncs-paths-refs/moon.yml")).unwrap();

            create_moon_command(fixture.path())
                .arg("run")
                .arg("syncs-paths-refs:test")
                .assert();

            assert_snapshot!(
                read_to_string(fixture.path().join("syncs-paths-refs/tsconfig.json")).unwrap()
            );
        }

        #[test]
        fn doesnt_map_paths_if_disabled() {
            let fixture = create_sandbox_with_git("typescript");

            create_moon_command(fixture.path())
                .arg("run")
                .arg("syncs-paths-refs:test")
                .assert();

            assert_snapshot!(
                read_to_string(fixture.path().join("syncs-paths-refs/tsconfig.json")).unwrap()
            );
        }
    }
}

mod workspace_overrides {
    use super::*;

    #[test]
    #[serial]
    fn can_override_version() {
        let fixture = create_sandbox_with_git("node");

        let assert = create_moon_command(fixture.path())
            .arg("run")
            .arg("base:version")
            .arg("versionOverride:version")
            .assert();

        let output = get_assert_output(&assert);

        assert!(predicate::str::contains("v14.0.0").eval(&output));
        assert!(predicate::str::contains("v16.1.0").eval(&output));

        assert.success();
    }
}
