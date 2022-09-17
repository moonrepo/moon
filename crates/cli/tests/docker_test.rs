use moon_utils::test::{create_moon_command, create_sandbox_with_git};

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

        create_moon_command(fixture.path())
            .arg("docker")
            .arg("scaffold")
            .arg("base")
            // Janky but works
            .arg("--include")
            .arg("system/*.sh")
            .arg("--include")
            .arg("system-windows/*.bat")
            .assert();

        let docker = fixture.join(".moon/docker/sources");

        assert!(docker.join("base").exists());
        assert!(docker.join("system/cwd.sh").exists());
        assert!(docker.join("system-windows/cwd.bat").exists());

        // Check that some others DO NOT exist
        assert!(!docker.join("node/cwd.js").exists());
    }
}
