use moon_utils::is_ci;
use moon_utils::test::{
    create_fixtures_skeleton_sandbox, create_moon_command_in, get_fixtures_dir,
};
use predicates::prelude::*;

mod run_script {
    use super::*;

    #[test]
    fn errors_if_no_project() {
        let fixture = create_fixtures_skeleton_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .args(["node", "run-script", "unknown"])
            .assert();

        assert.failure().stderr(predicate::str::contains(
            "This command must be ran within the context of a project.",
        ));
    }

    #[test]
    fn errors_for_unknown_script() {
        let fixture = create_fixtures_skeleton_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .args(["node", "run-script", "unknown", "--project", "npm"])
            .assert();

        assert
            .failure()
            .stderr(predicate::str::contains("Missing script: \"unknown\""));
    }

    #[test]
    fn runs_with_project_option() {
        let fixture = create_fixtures_skeleton_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .args(["node", "run-script", "test", "--project", "npm"])
            .assert();

        assert.success().stdout(predicate::str::contains("> test"));
    }

    #[test]
    fn runs_with_env_var() {
        let fixture = create_fixtures_skeleton_sandbox("node-npm");

        let assert = create_moon_command_in(fixture.path())
            .args(["node", "run-script", "test"])
            .env(
                "MOON_PROJECT_ROOT",
                get_fixtures_dir("node-npm").join("npm"),
            )
            .assert();

        assert.success().stdout(predicate::str::contains("> test"));
    }

    #[test]
    fn works_with_pnpm() {
        // This requires the toolchain to be installed, which may not be
        if is_ci() {
            return;
        }

        let fixture = create_fixtures_skeleton_sandbox("node-pnpm");

        let assert = create_moon_command_in(fixture.path())
            .args(["node", "run-script", "lint", "--project", "pnpm"])
            .assert();

        assert.success().stdout(predicate::str::contains("lint"));
    }

    #[test]
    fn works_with_yarn() {
        // This requires the toolchain to be installed, which may not be
        if is_ci() {
            return;
        }

        let fixture = create_fixtures_skeleton_sandbox("node-yarn");

        let assert = create_moon_command_in(fixture.path())
            .args(["node", "run-script", "build", "--project", "yarn"])
            .assert();

        assert.success().stdout(predicate::str::contains("build"));
    }
}
