use moon_utils::test::{create_moon_command, get_fixtures_dir};
use predicates::prelude::*;

mod run_script {
    use super::*;

    #[test]
    fn errors_if_no_project() {
        let assert = create_moon_command("node-npm")
            .args(["node", "run-script", "unknown"])
            .assert();

        assert.failure().stderr(predicate::str::contains(
            "This command must be ran within the context of a project.",
        ));
    }

    #[test]
    fn errors_for_unknown_script() {
        let assert = create_moon_command("node-npm")
            .args(["node", "run-script", "unknown", "--project", "npm"])
            .assert();

        assert
            .failure()
            .stderr(predicate::str::contains("Missing script: \"unknown\""));
    }

    #[test]
    fn runs_with_project_option() {
        let assert = create_moon_command("node-npm")
            .args(["node", "run-script", "test", "--project", "npm"])
            .assert();

        assert.success().stdout(predicate::str::contains("> test"));
    }

    #[test]
    fn runs_with_env_var() {
        let assert = create_moon_command("node-npm")
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
        let assert = create_moon_command("node-pnpm")
            .args(["node", "run-script", "lint", "--project", "pnpm"])
            .assert();

        assert.success().stdout(predicate::str::contains("lint"));
    }

    // Requires a lockfile... but works
    // #[test]
    // fn works_with_yarn() {
    //     let assert = create_moon_command("node-yarn1")
    //         .args(["node", "run-script", "build", "--project", "yarn"])
    //         .assert();

    //     assert.success().stdout(predicate::str::contains("> build"));
    // }
}
