use moon_app::commands::docker::DockerManifest;
use moon_common::Id;
use moon_config::{PartialWorkspaceConfig, PartialWorkspaceProjects};
use moon_test_utils::{
    create_sandbox_with_config, get_node_depman_fixture_configs, get_node_fixture_configs,
    get_projects_fixture_configs, predicates::prelude::*,
};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::json;
use std::{fs, path::Path};

fn write_manifest(path: &Path, id: &str) {
    fs::write(
        path.join("dockerManifest.json"),
        json::format(
            &DockerManifest {
                focused_projects: FxHashSet::from_iter([Id::raw(id)]),
                unfocused_projects: FxHashSet::default(),
            },
            false,
        )
        .unwrap(),
    )
    .unwrap()
}

mod scaffold_workspace {
    use super::*;

    #[test]
    fn copies_all_manifests() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("esbuild");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("dockerManifest.json").exists());
        assert!(docker.join("esbuild/package.json").exists());
        assert!(docker.join("lifecycles/package.json").exists());
        assert!(docker.join("swc/package.json").exists());
        assert!(docker.join("package.json").exists());
    }

    #[test]
    fn copies_moon_configs() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        // Test inherited configs
        fs::create_dir(sandbox.path().join(".moon/tasks")).unwrap();
        fs::write(sandbox.path().join(".moon/tasks/node.yml"), "{}").unwrap();

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("lifecycles");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join(".moon/tasks.yml").exists());
        assert!(docker.join(".moon/tasks/node.yml").exists());
        assert!(docker.join(".moon/toolchain.yml").exists());
        assert!(docker.join(".moon/workspace.yml").exists());
        assert!(docker.join("base/moon.yml").exists());
        assert!(docker.join("swc/moon.yml").exists());
        assert!(docker.join("version-override/moon.yml").exists());
    }

    #[test]
    fn copies_node_postinstalls() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("lifecycles");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("lifecycles/package.json").exists());
        assert!(docker.join("lifecycles/postinstall.mjs").exists());
    }

    #[test]
    fn copies_npm_files() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("npm");

        let sandbox = create_sandbox_with_config(
            "node-npm/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("npm");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("package-lock.json").exists());
    }

    #[test]
    fn copies_pnpm_files() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("pnpm");

        let sandbox = create_sandbox_with_config(
            "node-pnpm/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("pnpm");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("pnpm-lock.yaml").exists());
        assert!(docker.join("pnpm-workspace.yaml").exists());
    }

    #[test]
    fn copies_yarn_files() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn");

        let sandbox = create_sandbox_with_config(
            "node-yarn/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("yarn");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join(".yarnrc.yml").exists());
        assert!(docker.join("yarn.lock").exists());
    }

    #[test]
    fn copies_yarn1_files() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn1");

        let sandbox = create_sandbox_with_config(
            "node-yarn1/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("yarn1");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("yarn.lock").exists());
    }

    // TODO: Bun doesn't support Windows yet!
    #[cfg(not(windows))]
    #[test]
    fn copies_node_bun_files() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("bun");

        let sandbox = create_sandbox_with_config(
            "node-bun/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("bun");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("bun.lockb").exists());
    }

    #[test]
    fn copies_cargo_workspace_files() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([(
                Id::raw("rust"),
                ".".into(),
            )]))),
            ..PartialWorkspaceConfig::default()
        };

        let sandbox =
            create_sandbox_with_config("rust/workspaces", Some(workspace_config), None, None);

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("rust");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("dockerManifest.json").exists());
        assert!(docker.join("Cargo.toml").exists());
        assert!(docker.join("Cargo.lock").exists());
        assert!(docker.join("crates/bin-crate/Cargo.toml").exists());
        assert!(docker.join("crates/bin-crate/src/lib.rs").exists());
        assert!(docker.join("crates/bin-crate/src/main.rs").exists());
        assert!(docker.join("crates/path-deps/Cargo.toml").exists());
        assert!(docker.join("crates/path-deps/src/lib.rs").exists());
        assert!(docker.join("crates/path-deps/src/main.rs").exists());
    }

    #[test]
    fn copies_cargo_non_workspace_files() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([(
                Id::raw("rust"),
                ".".into(),
            )]))),
            ..PartialWorkspaceConfig::default()
        };

        let sandbox =
            create_sandbox_with_config("rust/project", Some(workspace_config), None, None);

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("rust");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("dockerManifest.json").exists());
        assert!(docker.join("Cargo.toml").exists());
        assert!(docker.join("Cargo.lock").exists());
        assert!(docker.join("src/lib.rs").exists());
        assert!(docker.join("src/main.rs").exists());
    }
}

mod scaffold_sources {
    use super::*;

    #[test]
    fn copies_project_and_deps() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("basic");
        });

        let docker = sandbox.path().join(".moon/docker/sources");

        assert!(docker.join("dockerManifest.json").exists());
        assert!(docker.join("basic/file.ts").exists());
        assert!(docker.join("no-config/empty").exists());

        // Check that some others DO NOT exist
        assert!(!docker.join("advanced").exists());
        assert!(!docker.join("deps").exists());
        assert!(!docker.join("tasks").exists());
    }

    #[test]
    fn copies_multiple_projects() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("tasks").arg("bar");
        });

        let docker = sandbox.path().join(".moon/docker/sources");

        assert!(docker.join("deps/bar").exists());
        assert!(docker.join("tasks").exists());

        // Check that some others DO NOT exist
        assert!(!docker.join("deps/foo").exists());
        assert!(!docker.join("deps/baz").exists());
    }

    #[test]
    fn doesnt_copy_node_modules() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.create_file("node_modules/root/file", "");
        sandbox.create_file("basic/node_modules/nested/file", "");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("basic");
        });

        let docker = sandbox.path().join(".moon/docker/sources");

        assert!(!docker.join("node_modules").exists());
        assert!(!docker.join("basic/node_nodules").exists());
    }

    #[test]
    fn doesnt_copy_rust_target() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        sandbox.create_file("target/root/file", "");
        sandbox.create_file("basic/target/nested/file", "");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("basic");
        });

        let docker = sandbox.path().join(".moon/docker/sources");

        assert!(!docker.join("target").exists());
        assert!(!docker.join("basic/target").exists());
    }
}

mod prune {
    use super::*;

    #[test]
    fn errors_missing_manifest() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        assert!(
            predicate::str::contains("Unable to continue, docker manifest missing.")
                .eval(&assert.output())
        );
    }
}

#[cfg(not(target_os = "linux"))]
mod prune_node {
    use super::*;

    #[test]
    fn focuses_for_npm() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("npm");

        let sandbox = create_sandbox_with_config(
            "node-npm/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox
            .run_moon(|cmd| {
                cmd.arg("docker").arg("prune");
            })
            .debug();

        // should exist
        assert!(sandbox.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("npm/node_modules").exists());
        assert!(
            !sandbox
                .path()
                .join("node_modules/babel-preset-solid")
                .exists()
        );

        // npm installs prod deps for unfocused
        // assert!(!sandbox.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_pnpm() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("pnpm");

        let sandbox = create_sandbox_with_config(
            "node-pnpm/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        // should exist
        assert!(sandbox.path().join("other/node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("pnpm/node_modules").exists());
        assert!(
            !sandbox
                .path()
                .join("node_modules/babel-preset-solid")
                .exists()
        );
        assert!(!sandbox.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_yarn() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn");

        let sandbox = create_sandbox_with_config(
            "node-yarn/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        // should exist
        assert!(sandbox.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("npm/node_modules").exists());
        assert!(
            !sandbox
                .path()
                .join("node_modules/babel-preset-solid")
                .exists()
        );
        assert!(!sandbox.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_yarn1() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn1");

        let sandbox = create_sandbox_with_config(
            "node-yarn1/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        // should exist
        assert!(sandbox.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("yarn/node_modules").exists());
        assert!(
            !sandbox
                .path()
                .join("node_modules/babel-preset-solid")
                .exists()
        );

        // yarn 1 does not support focusing
        // assert!(!sandbox.path().join("node_modules/react").exists());
    }

    // #[test]
    // fn focuses_for_node_bun() {
    //     let (workspace_config, toolchain_config, tasks_config) =
    //         get_node_depman_fixture_configs("bun");

    //     let sandbox = create_sandbox_with_config(
    //         "node-bun/workspaces",
    //         Some(workspace_config),
    //         Some(toolchain_config),
    //         Some(tasks_config),
    //     );

    //     write_manifest(sandbox.path(), "other");

    //     sandbox.run_moon(|cmd| {
    //         cmd.arg("docker").arg("prune");
    //     });

    //     // should exist
    //     assert!(sandbox.path().join("other/node_modules/solid-js").exists());

    //     // should not exist
    //     assert!(!sandbox.path().join("pnpm/node_modules").exists());
    //     assert!(!sandbox
    //         .path()
    //         .join("node_modules/babel-preset-solid")
    //         .exists());
    //     assert!(!sandbox.path().join("node_modules/react").exists());
    // }
}

mod setup {
    use super::*;

    #[test]
    fn errors_missing_manifest() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("setup");
        });

        assert!(
            predicate::str::contains("Unable to continue, docker manifest missing.")
                .eval(&assert.output())
        );
    }
}

mod setup_node {
    use super::*;

    #[test]
    fn installs_npm() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("npm");

        let sandbox = create_sandbox_with_config(
            "node-npm/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("setup");
        });

        // only check root because of workspaces
        assert!(sandbox.path().join("node_modules").exists());
    }

    #[test]
    fn installs_pnpm() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("pnpm");

        let sandbox = create_sandbox_with_config(
            "node-pnpm/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("setup");
        });

        // only check root because of workspaces
        assert!(sandbox.path().join("node_modules").exists());
    }

    #[test]
    fn installs_yarn() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn");

        let sandbox = create_sandbox_with_config(
            "node-yarn/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("setup");
        });

        // only check root because of workspaces
        assert!(sandbox.path().join("node_modules").exists());
    }

    #[test]
    fn installs_yarn1() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn1");

        let sandbox = create_sandbox_with_config(
            "node-yarn1/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("setup");
        });

        // only check root because of workspaces
        assert!(sandbox.path().join("node_modules").exists());
    }

    // TODO: Bun doesn't support Windows yet!
    #[cfg(not(windows))]
    #[test]
    fn installs_node_bun() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("bun");

        let sandbox = create_sandbox_with_config(
            "node-bun/workspaces",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("setup");
        });

        // only check root because of workspaces
        assert!(sandbox.path().join("node_modules").exists());
    }
}
