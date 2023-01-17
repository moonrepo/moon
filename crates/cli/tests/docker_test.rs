use moon_cli::commands::docker::DockerManifest;
use moon_test_utils::{
    create_sandbox_with_config, get_cases_fixture_configs, get_node_depman_fixture_configs,
    get_node_fixture_configs, get_projects_fixture_configs, predicates::prelude::*,
};
use rustc_hash::FxHashSet;
use std::{fs, path::Path};

fn write_manifest(path: &Path, id: &str) {
    fs::write(
        path.join("dockerManifest.json"),
        serde_json::to_string(&DockerManifest {
            focused_projects: FxHashSet::from_iter([id.to_owned()]),
            unfocused_projects: FxHashSet::default(),
        })
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
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("esbuild");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

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
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("lifecycles");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join(".moon/tasks.yml").exists());
        assert!(docker.join(".moon/toolchain.yml").exists());
        assert!(docker.join(".moon/workspace.yml").exists());
    }

    #[test]
    fn copies_node_postinstalls() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
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
            "node-npm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
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
            "node-pnpm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
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
            "node-yarn",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
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
            "node-yarn1",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("yarn1");
        });

        let docker = sandbox.path().join(".moon/docker/workspace");

        assert!(docker.join("yarn.lock").exists());
    }
}

mod scaffold_sources {
    use super::*;

    #[test]
    fn copies_project_and_deps() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("scaffold").arg("basic");
        });

        let docker = sandbox.path().join(".moon/docker/sources");

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
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
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
    fn can_include_more_files() {
        let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "cases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        sandbox.run_moon(|cmd| {
            cmd.arg("docker")
                .arg("scaffold")
                .arg("base")
                // Janky but works
                .arg("--include")
                .arg("outputs/generate.js")
                .arg("--include")
                .arg("passthrough-args/*.sh");
        });

        let docker = sandbox.path().join(".moon/docker/sources");

        assert!(docker.join("base").exists());
        assert!(docker.join("outputs/generate.js").exists());
        assert!(docker.join("passthrough-args/passthroughArgs.sh").exists());

        // Check that some others DO NOT exist
        assert!(!docker.join("output-styles/style.js").exists());
    }
}

mod prune {
    use super::*;

    #[test]
    fn errors_missing_manifest() {
        let (workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "node",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        assert!(
            predicate::str::contains("Unable to prune, docker manifest missing. Has it been scaffolded with `moon docker scaffold`?")
                .eval(&assert.output())
        );
    }
}

mod prune_node {
    use super::*;

    #[test]
    fn focuses_for_npm() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("npm");

        let sandbox = create_sandbox_with_config(
            "node-npm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        // should exist
        assert!(sandbox.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("npm/node_modules").exists());
        assert!(!sandbox
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());

        // npm installs prod deps for unfocused
        // assert!(!sandbox.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_pnpm() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("pnpm");

        let sandbox = create_sandbox_with_config(
            "node-pnpm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        // should exist
        assert!(sandbox.path().join("other/node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("pnpm/node_modules").exists());
        assert!(!sandbox
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());
        assert!(!sandbox.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_yarn() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn");

        let sandbox = create_sandbox_with_config(
            "node-yarn",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        // should exist
        assert!(sandbox.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("npm/node_modules").exists());
        assert!(!sandbox
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());
        assert!(!sandbox.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_yarn1() {
        let (workspace_config, toolchain_config, tasks_config) =
            get_node_depman_fixture_configs("yarn1");

        let sandbox = create_sandbox_with_config(
            "node-yarn1",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&tasks_config),
        );

        write_manifest(sandbox.path(), "other");

        sandbox.run_moon(|cmd| {
            cmd.arg("docker").arg("prune");
        });

        // should exist
        assert!(sandbox.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!sandbox.path().join("yarn/node_modules").exists());
        assert!(!sandbox
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());

        // yarn 1 does not support focusing
        // assert!(!sandbox.path().join("node_modules/react").exists());
    }
}
