use moon_test_utils2::create_moon_sandbox;
use starbase_sandbox::predicates::prelude::*;
use std::fs;

mod dockerfile {
    use super::*;

    #[test]
    fn errors_for_unknown_project() {
        let sandbox = create_moon_sandbox("dockerfile");

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["docker", "file", "missing", "--defaults"]);
        });

        assert.inner.stderr(predicate::str::contains(
            "No project has been configured with the identifier or alias missing.",
        ));
    }

    #[test]
    fn errors_for_unknown_build_task() {
        let sandbox = create_moon_sandbox("dockerfile");

        let assert = sandbox.run_bin(|cmd| {
            cmd.args([
                "docker",
                "file",
                "no-tasks",
                "--defaults",
                "--buildTask",
                "missing",
            ]);
        });

        assert.inner.stderr(predicate::str::contains(
            "Unknown task missing for project no-tasks.",
        ));
    }

    #[test]
    fn errors_for_unknown_start_task() {
        let sandbox = create_moon_sandbox("dockerfile");

        let assert = sandbox.run_bin(|cmd| {
            cmd.args([
                "docker",
                "file",
                "no-tasks",
                "--defaults",
                "--startTask",
                "missing",
            ]);
        });

        assert.inner.stderr(predicate::str::contains(
            "Unknown task missing for project no-tasks.",
        ));
    }

    #[test]
    fn can_use_defaults() {
        let sandbox = create_moon_sandbox("dockerfile");

        sandbox
            .run_bin(|cmd| {
                cmd.args(["docker", "file", "has-tasks", "--defaults"]);
            })
            .success();

        assert!(sandbox.path().join("has-tasks/Dockerfile").exists());
    }

    #[test]
    fn can_change_dest() {
        let sandbox = create_moon_sandbox("dockerfile");

        sandbox
            .run_bin(|cmd| {
                cmd.args([
                    "docker",
                    "file",
                    "has-tasks",
                    "./nested/Dockerfile.prod",
                    "--defaults",
                ]);
            })
            .success();

        assert!(
            sandbox
                .path()
                .join("has-tasks/nested/Dockerfile.prod")
                .exists()
        );
    }

    #[test]
    fn can_customize_with_args() {
        let sandbox = create_moon_sandbox("dockerfile");

        sandbox
            .run_bin(|cmd| {
                cmd.args([
                    "docker",
                    "file",
                    "has-tasks",
                    "--image",
                    "node:latest",
                    "--buildTask",
                    "build",
                    "--startTask",
                    "start",
                    "--no-prune",
                    "--no-toolchain",
                ]);
            })
            .success();

        let file = fs::read_to_string(sandbox.path().join("has-tasks/Dockerfile")).unwrap();

        assert!(file.contains("FROM node:latest"));
        assert!(file.contains("moon run has-tasks:build"));
        assert!(file.contains("moon run has-tasks:start"));
        assert!(!file.contains("moon docker prune"));
        assert!(file.contains("MOON_TOOLCHAIN_FORCE_GLOBALS=1"));
    }

    #[test]
    fn uses_docker_config() {
        let sandbox = create_moon_sandbox("dockerfile");

        sandbox
            .run_bin(|cmd| {
                cmd.args(["docker", "file", "with-config", "--defaults"]);
            })
            .success();

        let file = fs::read_to_string(sandbox.path().join("with-config/Dockerfile")).unwrap();

        assert!(file.contains("FROM oven/bun:latest"));
        assert!(file.contains("moon run with-config:compile"));
        assert!(file.contains("moon run with-config:serve"));
        assert!(file.contains("moon docker prune"));
    }
}
