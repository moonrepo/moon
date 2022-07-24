use moon_utils::test::{create_moon_command, create_sandbox, get_fixtures_dir};
use predicates::prelude::*;
use serial_test::serial;
use std::path::Path;

fn setup_toolchain(path: &Path) {
    let assert = create_moon_command(path).args(["setup"]).assert();

    assert.success();
}

mod run_script {
    use super::*;

    #[test]
    #[serial]
    fn errors_if_no_project() {
        let fixture = create_sandbox("node-npm");

        setup_toolchain(fixture.path());

        let assert = create_moon_command(fixture.path())
            .args(["node", "run-script", "unknown"])
            .assert();

        assert.failure().stderr(predicate::str::contains(
            "This command must be ran within the context of a project.",
        ));
    }

    #[test]
    #[serial]
    fn errors_for_unknown_script() {
        let fixture = create_sandbox("node-npm");

        setup_toolchain(fixture.path());

        let assert = create_moon_command(fixture.path())
            .args(["node", "run-script", "unknown", "--project", "npm"])
            .assert();

        assert
            .failure()
            .stderr(predicate::str::contains("missing script"));
    }

    #[test]
    #[serial]
    fn runs_with_project_option() {
        let fixture = create_sandbox("node-npm");

        setup_toolchain(fixture.path());

        let assert = create_moon_command(fixture.path())
            .args(["node", "run-script", "test", "--project", "npm"])
            .assert();

        assert.success().stdout(predicate::str::contains("> test"));
    }

    #[test]
    #[serial]
    fn runs_with_env_var() {
        let fixture = create_sandbox("node-npm");

        setup_toolchain(fixture.path());

        let assert = create_moon_command(fixture.path())
            .args(["node", "run-script", "test"])
            .env(
                "MOON_PROJECT_ROOT",
                get_fixtures_dir("node-npm").join("npm"),
            )
            .assert();

        assert.success().stdout(predicate::str::contains("> test"));
    }

    #[test]
    #[serial]
    fn works_with_pnpm() {
        // TODO: This passes locally but fails in CI...
        if cfg!(windows) && moon_utils::is_ci() {
            return;
        }

        let fixture = create_sandbox("node-pnpm");

        setup_toolchain(fixture.path());

        let assert = create_moon_command(fixture.path())
            .args(["node", "run-script", "lint", "--project", "pnpm"])
            .assert();

        assert.success().stdout(predicate::str::contains("lint"));
    }

    // TODO: Yarn requires the lockfile to exist and be accurate,
    // otherwise it crashes. We cant do this easily right now.
    // #[test]
    // #[serial]
    // fn works_with_yarn() {
    //     let fixture = create_sandbox("node-yarn");

    //     setup_toolchain(fixture.path());

    //     let assert = create_moon_command(fixture.path())
    //         .args(["node", "run-script", "build", "--project", "yarn"])
    //         .assert();

    //     assert.success().stdout(predicate::str::contains("build"));
    // }
}
