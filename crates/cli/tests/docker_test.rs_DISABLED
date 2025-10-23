use moon_app::commands::docker::DockerManifest;
use moon_common::Id;
use moon_test_utils::{create_sandbox_with_config, get_projects_fixture_configs};
use rustc_hash::FxHashSet;
use starbase_utils::json;
use std::{fs, path::Path};

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

#[cfg(windows)]
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

#[cfg(windows)]
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
