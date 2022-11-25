use moon_cli::commands::docker::DockerManifest;
use moon_utils::test::{
    create_moon_command, create_sandbox, create_sandbox_with_git, get_assert_output,
};
use predicates::prelude::*;
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
        let fixture = create_sandbox_with_git("node");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("esbuild")
            .assert();

        let docker = fixture.join(".moon/docker/workspace");

        assert!(docker.join("esbuild/package.json").exists());
        assert!(docker.join("lifecycles/package.json").exists());
        assert!(docker.join("swc/package.json").exists());
        assert!(docker.join("package.json").exists());
    }

    #[test]
    fn copies_moon_configs() {
        let fixture = create_sandbox_with_git("node");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("lifecycles")
            .assert();

        let docker = fixture.join(".moon/docker/workspace");

        assert!(docker.join(".moon/project.yml").exists());
        assert!(docker.join(".moon/workspace.yml").exists());
    }

    #[test]
    fn copies_node_postinstalls() {
        let fixture = create_sandbox_with_git("node");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("lifecycles")
            .assert();

        let docker = fixture.join(".moon/docker/workspace");

        assert!(docker.join("lifecycles/package.json").exists());
        assert!(docker.join("lifecycles/postinstall.mjs").exists());
    }

    #[test]
    fn copies_npm_files() {
        let fixture = create_sandbox_with_git("node-npm");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("npm")
            .assert();

        let docker = fixture.join(".moon/docker/workspace");

        assert!(docker.join("package-lock.json").exists());
    }

    #[test]
    fn copies_pnpm_files() {
        let fixture = create_sandbox_with_git("node-pnpm");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("pnpm")
            .assert();

        let docker = fixture.join(".moon/docker/workspace");

        assert!(docker.join("pnpm-lock.yaml").exists());
        assert!(docker.join("pnpm-workspace.yaml").exists());
    }

    #[test]
    fn copies_yarn_files() {
        let fixture = create_sandbox_with_git("node-yarn");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("yarn")
            .assert();

        let docker = fixture.join(".moon/docker/workspace");

        assert!(docker.join(".yarnrc.yml").exists());
        assert!(docker.join("yarn.lock").exists());
    }

    #[test]
    fn copies_yarn1_files() {
        let fixture = create_sandbox_with_git("node-yarn1");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("yarn")
            .assert();

        let docker = fixture.join(".moon/docker/workspace");

        assert!(docker.join("yarn.lock").exists());
    }
}

mod scaffold_sources {
    use super::*;

    #[test]
    fn copies_project_and_deps() {
        let fixture = create_sandbox_with_git("projects");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("basic")
            .assert();

        let docker = fixture.join(".moon/docker/sources");

        assert!(docker.join("basic/file.ts").exists());
        assert!(docker.join("no-config/empty").exists());

        // Check that some others DO NOT exist
        assert!(!docker.join("advanced").exists());
        assert!(!docker.join("deps").exists());
        assert!(!docker.join("langs").exists());
        assert!(!docker.join("tasks").exists());
    }

    #[test]
    fn copies_multiple_projects() {
        let fixture = create_sandbox_with_git("projects");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("js")
            .arg("bar")
            .assert();

        let docker = fixture.join(".moon/docker/sources");

        assert!(docker.join("langs/js").exists());
        assert!(docker.join("deps/bar").exists());

        // Check that some others DO NOT exist
        assert!(!docker.join("langs/ts").exists());
        assert!(!docker.join("langs/bash").exists());
        assert!(!docker.join("deps/foo").exists());
        assert!(!docker.join("deps/baz").exists());
    }

    #[test]
    fn can_include_more_files() {
        let fixture = create_sandbox_with_git("cases");

        let assert = create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("base")
            // Janky but works
            .arg("--include")
            .arg("outputs/generate.js")
            .arg("--include")
            .arg("passthrough-args/*.sh")
            .assert();

        let docker = fixture.join(".moon/docker/sources");

        moon_utils::test::debug_sandbox(&fixture, &assert);

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
        let fixture = create_sandbox("node");

        let assert = create_moon_command(fixture.path())
            .arg("docker")
            .arg("prune")
            .assert();

        assert!(
            predicate::str::contains("Unable to prune, docker manifest missing. Has it been scaffolded with `moon docker scaffold`?")
                .eval(&get_assert_output(&assert))
        );
    }
}

mod prune_node {
    use super::*;

    #[test]
    fn focuses_for_npm() {
        let fixture = create_sandbox("node-npm");

        write_manifest(fixture.path(), "other");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("prune")
            .assert();

        // should exist
        assert!(fixture.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!fixture.path().join("npm/node_modules").exists());
        assert!(!fixture
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());

        // npm installs prod deps for unfocused
        // assert!(!fixture.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_pnpm() {
        let fixture = create_sandbox("node-pnpm");

        write_manifest(fixture.path(), "other");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("prune")
            .assert();

        // should exist
        assert!(fixture.path().join("other/node_modules/solid-js").exists());

        // should not exist
        assert!(!fixture.path().join("pnpm/node_modules").exists());
        assert!(!fixture
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());
        assert!(!fixture.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_yarn() {
        let fixture = create_sandbox("node-yarn");

        write_manifest(fixture.path(), "other");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("prune")
            .assert();

        // should exist
        assert!(fixture.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!fixture.path().join("npm/node_modules").exists());
        assert!(!fixture
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());
        assert!(!fixture.path().join("node_modules/react").exists());
    }

    #[test]
    fn focuses_for_yarn1() {
        let fixture = create_sandbox("node-yarn1");

        write_manifest(fixture.path(), "other");

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("prune")
            .assert();

        // should exist
        assert!(fixture.path().join("node_modules/solid-js").exists());

        // should not exist
        assert!(!fixture.path().join("yarn/node_modules").exists());
        assert!(!fixture
            .path()
            .join("node_modules/babel-preset-solid")
            .exists());

        // yarn 1 does not support focusing
        // assert!(!fixture.path().join("node_modules/react").exists());
    }
}
