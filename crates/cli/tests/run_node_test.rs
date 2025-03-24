use moon_common::Id;
use moon_config::{
    NodePackageManager, NodeVersionFormat, NodeVersionManager, PartialNodeConfig,
    PartialWorkspaceProjects, PartialYarnConfig,
};
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, get_node_depman_fixture_configs,
    get_node_fixture_configs, get_typescript_fixture_configs, predicates::prelude::*,
};
use proto_core::UnresolvedVersionSpec;
use rustc_hash::FxHashMap;
use starbase_utils::string_vec;
use std::fs::{self, read_to_string};

fn node_sandbox() -> Sandbox {
    node_sandbox_with_config(|_| {})
}

fn node_sandbox_with_config<C>(callback: C) -> Sandbox
where
    C: FnOnce(&mut PartialNodeConfig),
{
    let (workspace_config, mut toolchain_config, tasks_config) = get_node_fixture_configs();

    if let Some(node_config) = &mut toolchain_config.node {
        callback(node_config);
    }

    let sandbox = create_sandbox_with_config(
        "node",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

fn depman_sandbox(depman: &str) -> Sandbox {
    let (workspace_config, toolchain_config, tasks_config) =
        get_node_depman_fixture_configs(depman);

    let sandbox = create_sandbox_with_config(
        format!("node-{depman}/workspaces"),
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

fn depman_non_workspaces_sandbox(depman: &str) -> Sandbox {
    let (mut workspace_config, toolchain_config, tasks_config) =
        get_node_depman_fixture_configs(depman);

    workspace_config.projects = Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([(
        Id::raw("root"),
        ".".into(),
    )])));

    let sandbox = create_sandbox_with_config(
        format!("node-{depman}/project"),
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

#[test]
fn runs_package_managers() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:npm");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn runs_standard_script() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:standard");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn runs_cjs_files() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn runs_mjs_files() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:mjs");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn supports_top_level_await() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:topLevelAwait");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_process_exit_zero() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:processExitZero");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_process_exit_nonzero() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:processExitNonZero");
    });

    if cfg!(windows) {
        assert.code(1);
    } else {
        assert_snapshot!(assert.output());
    }
}

#[test]
fn handles_process_exit_code_zero() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:exitCodeZero");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn handles_process_exit_code_nonzero() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:exitCodeNonZero");
    });

    if cfg!(windows) {
        assert.code(1);
    } else {
        assert_snapshot!(assert.output());
    }
}

#[test]
fn handles_throw_error() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:throwError");
    });

    let output = assert.output();

    // Output contains file paths that we cant snapshot
    assert!(predicate::str::contains("Error: Oops").eval(&output));
}

#[test]
fn handles_unhandled_promise() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:unhandledPromise");
    });

    if cfg!(windows) {
        assert.code(1);
    } else {
        assert_snapshot!(assert.output());
    }
}

#[test]
fn passes_args_through() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run")
            .arg("node:passthroughArgs")
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

    assert!(
        predicate::str::contains("Args: -aBc --opt value --optCamel=value").eval(&assert.output())
    );
}

#[test]
fn passes_args_to_the_node_bin() {
    let sandbox = node_sandbox_with_config(|cfg| {
        cfg.bin_exec_args = Some(string_vec!["--preserve-symlinks"]);
    });

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run")
            .arg("node:binExecArgs")
            .arg("--")
            .arg("--extraArg");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn sets_env_vars() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:envVars");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn inherits_moon_env_vars() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:envVarsMoon");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn forces_cache_to_write_only() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:envVarsMoon").arg("--updateCache");
    });

    assert!(predicate::str::contains("MOON_CACHE=write").eval(&assert.output()));
}

#[test]
fn runs_from_project_root() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:runFromProject");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn runs_from_workspace_root() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:runFromWorkspace");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn runs_npm_bin_from_workspace_root() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:runFromWorkspaceBin");
    });

    assert_snapshot!(assert.output());
}

#[test]
fn retries_on_failure_till_count() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:retryCount");
    });

    let output = assert.output();

    assert!(predicate::str::contains("exit code 1").eval(&output));
}

#[test]
fn can_run_many_targets() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs").arg("node:mjs");
    });

    let output = assert.output();

    assert!(predicate::str::contains("node:cjs | stdout").eval(&output));
    assert!(predicate::str::contains("node:mjs | stdout").eval(&output));
    assert!(predicate::str::contains("node:cjs | stderr").eval(&output));
    assert!(predicate::str::contains("node:mjs | stderr").eval(&output));
}

#[test]
fn avoids_postinstall_recursion() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("postinstallRecursion:noop");
    });

    let output = assert.output();

    assert!(
        predicate::str::contains("postinstallRecursion:noop")
            .count(1)
            .eval(&output)
    );

    assert.success();
}

#[test]
fn can_exec_global_bin_as_child_process() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:execBinSelf");
    });

    let output = assert.output();

    assert!(predicate::str::contains("execBinSelf").eval(&output));
    assert!(predicate::str::contains("v").eval(&output)); // Version not deterministic
}

#[test]
fn can_exec_global_bin_as_child_process_from_postinstall() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("postinstall:noop");
    });

    assert.success();
}

#[test]
fn runs_script_task() {
    let sandbox = node_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:viaScript");
    });

    assert_snapshot!(assert.output());
}

mod install_deps {
    use super::*;

    #[test]
    fn installs_on_first_run() {
        let sandbox = node_sandbox();

        assert!(!sandbox.path().join("node_modules").exists());

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:standard")
                .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT");
        });

        let output = assert.output();

        assert!(sandbox.path().join("node_modules").exists());

        assert!(predicate::str::contains("added").eval(&output));
        assert!(predicate::str::contains("packages").eval(&output));
    }

    #[test]
    fn doesnt_reinstall_on_second_run() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:standard")
                .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT");
        });

        let output1 = assert.output();

        assert!(predicate::str::contains("added").eval(&output1));
        assert!(predicate::str::contains("packages").eval(&output1));

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:standard")
                .env_remove("MOON_TEST_HIDE_INSTALL_OUTPUT");
        });

        let output2 = assert.output();

        assert!(!predicate::str::contains("added").eval(&output2));
        assert!(!predicate::str::contains("packages").eval(&output2));
    }

    #[test]
    fn creates_tool_state_cache() {
        let sandbox = node_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:standard");
        });

        assert!(
            sandbox
                .path()
                // 18.0.0
                .join(".moon/cache/states/setupToolchain-node-18.0.0.json")
                .exists()
        );
    }

    #[test]
    fn installs_deps_into_each_project_when_not_using_workspaces() {
        let (workspace_config, toolchain_config, tasks_config) = get_typescript_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node-non-workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("foo:noop")
                .arg("bar:noop")
                .arg("baz:noop");
        });

        assert!(
            predicate::str::contains("npm install")
                .count(3)
                .eval(&assert.output())
        );

        assert!(sandbox.path().join("foo/package-lock.json").exists());
        assert!(sandbox.path().join("bar/package-lock.json").exists());
        assert!(sandbox.path().join("baz/package-lock.json").exists());
    }
}

mod engines {
    use super::*;

    #[test]
    fn adds_engines_constraint() {
        let sandbox = node_sandbox_with_config(|cfg| {
            cfg.add_engines_constraint = Some(true);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:standard");
        });

        assert_snapshot!(read_to_string(sandbox.path().join("package.json")).unwrap());
    }

    #[test]
    fn doesnt_add_engines_constraint() {
        let sandbox = node_sandbox_with_config(|cfg| {
            cfg.add_engines_constraint = Some(false);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:standard");
        });

        assert_snapshot!(read_to_string(sandbox.path().join("package.json")).unwrap());
    }
}

mod version_manager {
    use super::*;

    #[test]
    fn adds_no_file_by_default() {
        let sandbox = node_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:standard");
        });

        assert!(!sandbox.path().join(".nvmrc").exists());
        assert!(!sandbox.path().join(".node-version").exists());
    }

    #[test]
    fn adds_nvmrc_file() {
        let sandbox = node_sandbox_with_config(|cfg| {
            cfg.sync_version_manager_config = Some(NodeVersionManager::Nvm);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:standard");
        });

        assert!(sandbox.path().join(".nvmrc").exists());

        assert_eq!(
            read_to_string(sandbox.path().join(".nvmrc")).unwrap(),
            "18.0.0"
        );
    }

    #[test]
    fn adds_nodenv_file() {
        let sandbox = node_sandbox_with_config(|cfg| {
            cfg.sync_version_manager_config = Some(NodeVersionManager::Nodenv);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:standard");
        });

        assert!(sandbox.path().join(".node-version").exists());

        assert_eq!(
            read_to_string(sandbox.path().join(".node-version")).unwrap(),
            "18.0.0"
        );
    }
}

mod sync_depends_on {
    use super::*;

    fn test_depends_on_format(format: NodeVersionFormat) {
        let sandbox = node_sandbox_with_config(|cfg| {
            // Other pm's don't support all formats
            cfg.package_manager = Some(NodePackageManager::Yarn);
            cfg.yarn = Some(PartialYarnConfig {
                version: Some(UnresolvedVersionSpec::parse("1.22.0").unwrap()),
                ..PartialYarnConfig::default()
            });
            cfg.sync_project_workspace_dependencies = Some(true);
            cfg.dependency_version_format = Some(format);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("dependsOn:standard");
        });

        // deps-c does not have a `package.json` on purpose
        assert_snapshot!(
            format!("format_{format:?}"),
            read_to_string(sandbox.path().join("depends-on/package.json")).unwrap()
        );
    }

    #[test]
    fn syncs_as_file_dependency() {
        test_depends_on_format(NodeVersionFormat::File);
    }

    #[test]
    fn syncs_as_link_dependency() {
        test_depends_on_format(NodeVersionFormat::Link);
    }

    #[test]
    fn syncs_as_star_dependency() {
        test_depends_on_format(NodeVersionFormat::Star);
    }

    #[test]
    fn syncs_as_version_dependency() {
        test_depends_on_format(NodeVersionFormat::Version);
    }

    #[test]
    fn syncs_as_version_caret_dependency() {
        test_depends_on_format(NodeVersionFormat::VersionCaret);
    }

    #[test]
    fn syncs_as_version_tilde_dependency() {
        test_depends_on_format(NodeVersionFormat::VersionTilde);
    }

    #[test]
    fn syncs_as_workspace_dependency() {
        test_depends_on_format(NodeVersionFormat::Workspace);
    }

    #[test]
    fn syncs_as_workspace_caret_dependency() {
        test_depends_on_format(NodeVersionFormat::WorkspaceCaret);
    }

    #[test]
    fn syncs_as_workspace_tilde_dependency() {
        test_depends_on_format(NodeVersionFormat::WorkspaceTilde);
    }

    #[test]
    fn syncs_depends_on_with_scopes() {
        let sandbox = node_sandbox_with_config(|cfg| {
            // Other pm's don't support all formats
            cfg.package_manager = Some(NodePackageManager::Yarn);
            cfg.yarn = Some(PartialYarnConfig {
                version: Some(UnresolvedVersionSpec::parse("1.22.0").unwrap()),
                ..PartialYarnConfig::default()
            });
            cfg.sync_project_workspace_dependencies = Some(true);
        });

        sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("dependsOnScopes:standard");
        });

        // deps-c does not have a `package.json` on purpose
        assert_snapshot!(
            read_to_string(sandbox.path().join("depends-on-scopes/package.json")).unwrap()
        );
    }
}

mod npm {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let sandbox = depman_sandbox("npm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("npm:version");
        });

        assert!(predicate::str::contains("8.0.0").eval(&assert.output()));
    }

    #[test]
    fn can_install_a_dep() {
        let sandbox = depman_sandbox("npm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("npm:installDep");
        });

        assert.success();
    }

    #[test]
    fn can_run_a_script() {
        let sandbox = depman_sandbox("npm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("npm:runScript");
        });

        assert!(predicate::str::contains("test").eval(&assert.output()));

        assert.success();
    }

    #[test]
    fn can_run_a_deps_bin() {
        let sandbox = depman_sandbox("npm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("npm:runDep");
        });

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&assert.output())
        );

        assert.success();
    }

    #[test]
    fn installs_deps_in_non_workspace_project() {
        let sandbox = depman_sandbox("npm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("notInWorkspace:noop")
                // Run other package so we can see both working
                .arg("npm:noop");
        });

        assert!(predicate::str::contains("npm install").eval(&assert.output()));

        assert!(sandbox.path().join("package-lock.json").exists());
        assert!(
            sandbox
                .path()
                .join("not-in-workspace/package-lock.json")
                .exists()
        );
        assert!(
            sandbox
                .path()
                .join("not-in-workspace/node_modules")
                .exists()
        );

        assert.success();
    }

    #[test]
    fn works_in_non_workspaces_project() {
        let sandbox = depman_non_workspaces_sandbox("npm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:version");
        });

        assert!(predicate::str::contains("8.0.0").eval(&assert.output()));
    }

    #[test]
    fn runs_script_task() {
        let sandbox = depman_sandbox("npm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("npm:viaScript");
        });

        assert_snapshot!(assert.output());
    }
}

mod pnpm {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let sandbox = depman_sandbox("pnpm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("pnpm:version");
        });

        assert!(predicate::str::contains("7.5.0").eval(&assert.output()));
    }

    #[test]
    fn can_install_a_dep() {
        let sandbox = depman_sandbox("pnpm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("pnpm:installDep");
        });

        assert.success();
    }

    #[test]
    fn can_run_a_script() {
        let sandbox = depman_sandbox("pnpm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("pnpm:runScript");
        });

        assert!(predicate::str::contains("lint").eval(&assert.output()));

        assert.success();
    }

    #[test]
    fn can_run_a_deps_bin_isolated() {
        let sandbox = depman_sandbox("pnpm");

        sandbox.create_file(".npmrc", "node-linker=isolated");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("pnpm:runDep");
        });

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&assert.output())
        );

        assert.success();
    }

    #[test]
    fn can_run_a_deps_bin_hoisted() {
        let sandbox = depman_sandbox("pnpm");

        sandbox.create_file(".npmrc", "node-linker=hoisted");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("pnpm:runDep");
        });

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&assert.output())
        );

        assert.success();
    }

    // NOTE: pnpm does not support nested lockfiles.
    // #[test]
    // fn installs_deps_in_non_workspace_project() {
    //     let sandbox = depman_sandbox("pnpm");

    //     let assert = create_moon_command(sandbox.path())
    //         .arg("run")
    //         .arg("notInWorkspace:noop")
    //         // Run other package so we can see both working
    //         .arg("pnpm:noop")
    //         .assert();

    //     assert_snapshot!(assert.output());

    //     assert!(sandbox.path().join("pnpm-lock.yaml").exists());
    //     assert!(sandbox
    //         .path()
    //         .join("not-in-workspace/pnpm-lock.yaml")
    //         .exists());
    //     assert!(sandbox
    //         .path()
    //         .join("not-in-workspace/node_modules")
    //         .exists());

    //     assert.success();
    // }

    #[test]
    fn works_in_non_workspaces_project() {
        let sandbox = depman_non_workspaces_sandbox("pnpm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:version");
        });

        assert!(predicate::str::contains("7.5.0").eval(&assert.output()));
    }

    #[test]
    fn runs_script_task() {
        let sandbox = depman_sandbox("pnpm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("pnpm:viaScript");
        });

        assert_snapshot!(assert.output());
    }
}

mod yarn1 {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let sandbox = depman_sandbox("yarn1");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn1:version");
        });

        assert!(predicate::str::contains("1.22.0").eval(&assert.output()));
    }

    #[test]
    fn can_install_a_dep() {
        let sandbox = depman_sandbox("yarn1");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn1:installDep");
        });

        assert.success();
    }

    #[test]
    fn can_run_a_script() {
        let sandbox = depman_sandbox("yarn1");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn1:runScript");
        });

        assert!(predicate::str::contains("build").eval(&assert.output()));

        assert.success();
    }

    #[test]
    fn can_run_a_deps_bin() {
        let sandbox = depman_sandbox("yarn1");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn1:runDep");
        });

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&assert.output())
        );

        assert.success();
    }

    #[test]
    fn installs_deps_in_non_workspace_project() {
        let sandbox = depman_sandbox("yarn1");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("notInWorkspace:noop")
                // Run other package so we can see both working
                .arg("yarn1:noop");
        });

        assert!(predicate::str::contains("yarn install").eval(&assert.output()));

        assert!(sandbox.path().join("yarn.lock").exists());
        assert!(sandbox.path().join("not-in-workspace/yarn.lock").exists());
        assert!(
            sandbox
                .path()
                .join("not-in-workspace/node_modules")
                .exists()
        );

        assert.success();
    }

    #[test]
    fn works_in_non_workspaces_project() {
        let sandbox = depman_non_workspaces_sandbox("yarn1");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:version");
        });

        assert!(predicate::str::contains("1.22.0").eval(&assert.output()));
    }

    #[test]
    fn runs_script_task() {
        let sandbox = depman_sandbox("yarn1");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn1:viaScript");
        });

        assert_snapshot!(assert.output());
    }
}

mod yarn {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let sandbox = depman_sandbox("yarn");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn:version");
        });

        assert!(predicate::str::contains("3.3.0").eval(&assert.output()));
    }

    #[test]
    fn can_install_a_dep() {
        let sandbox = depman_sandbox("yarn");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn:installDep");
        });

        assert.success();
    }

    #[test]
    fn can_run_a_script() {
        let sandbox = depman_sandbox("yarn");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn:runScript");
        });

        assert!(predicate::str::contains("build").eval(&assert.output()));

        assert.success();
    }

    #[test]
    fn can_run_a_deps_bin() {
        let sandbox = depman_sandbox("yarn");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn:runDep");
        });

        assert!(
            predicate::str::contains("All matched files use Prettier code style!")
                .eval(&assert.output())
        );

        assert.success();
    }

    #[test]
    fn installs_deps_in_non_workspace_project() {
        let sandbox = depman_sandbox("yarn");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("notInWorkspace:noop")
                // Run other package so we can see both working
                .arg("yarn:noop");
        });

        assert!(predicate::str::contains("yarn install").eval(&assert.output()));

        assert!(sandbox.path().join("yarn.lock").exists());
        assert!(sandbox.path().join("not-in-workspace/yarn.lock").exists());
        assert!(
            sandbox
                .path()
                .join("not-in-workspace/node_modules")
                .exists()
        );

        assert.success();
    }

    #[test]
    fn works_in_non_workspaces_project() {
        let sandbox = depman_non_workspaces_sandbox("yarn");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:version");
        });

        assert!(predicate::str::contains("3.3.0").eval(&assert.output()));
    }

    #[test]
    fn runs_script_task() {
        let sandbox = depman_sandbox("yarn");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("yarn:viaScript");
        });

        assert_snapshot!(assert.output());
    }
}

mod bun {
    use super::*;

    #[test]
    fn installs_correct_version() {
        let sandbox = depman_sandbox("bun");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:version");
        });

        assert!(predicate::str::contains("bun:version").eval(&assert.output()));
    }

    #[test]
    fn can_install_a_dep() {
        let sandbox = depman_sandbox("bun");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:installDep");
        });

        assert.success();
    }

    #[test]
    fn can_run_a_script() {
        let sandbox = depman_sandbox("bun");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:runScript");
        });

        assert!(predicate::str::contains("test").eval(&assert.output()));

        assert.success();
    }

    #[test]
    fn installs_deps_in_non_workspace_project() {
        let sandbox = depman_sandbox("bun");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("notInWorkspace:noop")
                // Run other package so we can see both working
                .arg("bun:noop")
                // Makes output deterministic
                .arg("--concurrency")
                .arg("1");
        });

        assert!(sandbox.path().join("bun.lockb").exists());
        assert!(sandbox.path().join("not-in-workspace/bun.lockb").exists());
        assert!(
            sandbox
                .path()
                .join("not-in-workspace/node_modules")
                .exists()
        );

        assert.success();
    }

    #[test]
    fn works_in_non_workspaces_project() {
        let sandbox = depman_non_workspaces_sandbox("pnpm");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("root:version");
        });

        assert!(predicate::str::contains("7.5.0").eval(&assert.output()));
    }

    #[test]
    fn runs_script_task() {
        let sandbox = depman_sandbox("bun");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("bun:viaScript");
        });

        assert_snapshot!(assert.output());
    }
}

mod profile {
    use super::*;

    #[test]
    fn record_a_cpu_profile() {
        let sandbox = node_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("--profile")
                .arg("cpu")
                .arg("node:standard");
        });

        let profile = sandbox
            .path()
            .join(".moon/cache/states/node/standard/snapshot.cpuprofile");

        assert!(profile.exists());
    }

    #[test]
    fn record_a_heap_profile() {
        let sandbox = node_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("--profile")
                .arg("heap")
                .arg("node:standard");
        });

        let profile = sandbox
            .path()
            .join(".moon/cache/states/node/standard/snapshot.heapprofile");

        assert!(profile.exists());
    }
}

mod aliases {
    use super::*;
    use moon_test_utils::get_project_graph_aliases_fixture_configs;

    #[test]
    fn runs_via_package_name() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_project_graph_aliases_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "project-graph/aliases",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("@scope/pkg-foo:standard");
        });

        assert_snapshot!(assert.output());
    }
}

mod non_js_bins {
    use super::*;
    use std::fs;

    #[test]
    fn works_with_esbuild() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("esbuild:build");
        });

        assert_eq!(
            fs::read_to_string(sandbox.path().join("esbuild/output.js")).unwrap(),
            "(() => {\n  // input.js\n  var ESBUILD = \"esbuild\";\n})();\n"
        );

        assert.success();
    }

    #[test]
    fn works_with_swc() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("swc:build");
        });

        assert_eq!(
            fs::read_to_string(sandbox.path().join("swc/output.js")).unwrap(),
            "export var SWC = \"swc\";\n\n\n//# sourceMappingURL=output.js.map"
        );

        assert.success();
    }
}

mod workspace_overrides {
    use super::*;

    #[test]
    fn can_override_version() {
        let sandbox = node_sandbox_with_config(|cfg| {
            cfg.dedupe_on_lockfile_change = Some(false);
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:version")
                .arg("versionOverride:version");
        });

        let output = assert.output();

        assert!(predicate::str::contains("v18.0.0").eval(&output));
        assert!(predicate::str::contains("v19.0.0").eval(&output));

        assert.success();
    }
}

mod affected_files {
    use super::*;

    #[test]
    fn all_files_when_not_affected() {
        let sandbox = node_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:affectedFiles");
        });

        let mut files = fs::read_dir(sandbox.path().join("base"))
            .unwrap()
            .map(|f| {
                f.unwrap()
                    .path()
                    .strip_prefix(sandbox.path().join("base"))
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

        assert!(predicate::str::contains(format!("Args: {}\n", args)).eval(&output));
        assert!(predicate::str::contains(format!("Env: {}\n", envs)).eval(&output));
    }

    #[test]
    fn uses_rel_paths_when_affected() {
        let sandbox = node_sandbox();

        sandbox.create_file("base/input1.js", "");
        sandbox.create_file("base/input2.js", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run").arg("node:affectedFiles").arg("--affected");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Args: ./input1.js ./input2.js\n").eval(&output));
        assert!(predicate::str::contains("Env: input1.js,input2.js\n").eval(&output));
    }

    #[test]
    fn sets_args_only() {
        let sandbox = node_sandbox();

        sandbox.create_file("base/input1.js", "");
        sandbox.create_file("base/input2.js", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:affectedFilesArgs")
                .arg("--affected");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Args: ./input1.js ./input2.js\n").eval(&output));
        assert!(predicate::str::contains("Env: \n").eval(&output));
    }

    #[test]
    fn sets_env_var_only() {
        let sandbox = node_sandbox();

        sandbox.create_file("base/input1.js", "");
        sandbox.create_file("base/input2.js", "");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("run")
                .arg("node:affectedFilesEnvVar")
                .arg("--affected");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Args: \n").eval(&output));
        assert!(predicate::str::contains("Env: input1.js,input2.js\n").eval(&output));
    }
}
