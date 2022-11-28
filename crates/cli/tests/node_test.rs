use moon_test_utils::{
    create_sandbox_with_config, get_fixtures_path, get_node_depman_fixture_configs,
};
use predicates::prelude::*;
use serial_test::serial;

mod run_script {
    use super::*;

    #[test]
    #[serial]
    fn errors_if_no_project() {
        let (workspace_config, toolchain_config, projects_config) =
            get_node_depman_fixture_configs("npm");

        let mut sandbox = create_sandbox_with_config(
            "node-npm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["node", "run-script", "unknown"]);
        });

        assert.failure().stderr(predicate::str::contains(
            "This command must be ran within the context of a project.",
        ));
    }

    #[test]
    #[serial]
    fn errors_for_unknown_script() {
        let (workspace_config, toolchain_config, projects_config) =
            get_node_depman_fixture_configs("npm");

        let mut sandbox = create_sandbox_with_config(
            "node-npm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["node", "run-script", "unknown", "--project", "npm"]);
        });

        assert
            .failure()
            .stderr(predicate::str::contains("Missing script"));
    }

    #[test]
    #[serial]
    fn runs_with_project_option() {
        let (workspace_config, toolchain_config, projects_config) =
            get_node_depman_fixture_configs("npm");

        let mut sandbox = create_sandbox_with_config(
            "node-npm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["node", "run-script", "test", "--project", "npm"]);
        });

        assert.success().stdout(predicate::str::contains("> test"));
    }

    #[test]
    #[serial]
    fn runs_with_env_var() {
        let (workspace_config, toolchain_config, projects_config) =
            get_node_depman_fixture_configs("npm");

        let mut sandbox = create_sandbox_with_config(
            "node-npm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["node", "run-script", "test"])
                .env("MOON_PROJECT_ROOT", get_fixtures_path("node-npm/base"));
        });

        assert.success().stdout(predicate::str::contains("> test"));
    }

    #[test]
    #[serial]
    fn works_with_pnpm() {
        let (workspace_config, toolchain_config, projects_config) =
            get_node_depman_fixture_configs("pnpm");

        let mut sandbox = create_sandbox_with_config(
            "node-pnpm",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["node", "run-script", "lint", "--project", "pnpm"]);
        });

        assert.success().stdout(predicate::str::contains("lint"));
    }

    #[test]
    #[serial]
    fn works_with_yarn() {
        let (workspace_config, toolchain_config, projects_config) =
            get_node_depman_fixture_configs("yarn");

        let mut sandbox = create_sandbox_with_config(
            "node-yarn",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["node", "run-script", "build", "--project", "yarn"]);
        });

        assert.success().stdout(predicate::str::contains("build"));
    }

    #[test]
    #[serial]
    fn works_with_yarn1() {
        let (workspace_config, toolchain_config, projects_config) =
            get_node_depman_fixture_configs("yarn1");

        let mut sandbox = create_sandbox_with_config(
            "node-yarn1",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["node", "run-script", "build", "--project", "yarn"]);
        });

        assert.success().stdout(predicate::str::contains("build"));
    }
}
