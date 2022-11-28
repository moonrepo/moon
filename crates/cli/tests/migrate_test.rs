use insta::assert_snapshot;
use moon_test_utils::create_sandbox;
use predicates::str::contains;
use std::fs;

mod from_package_json {
    use super::*;

    #[test]
    fn dirty_repository_raises_an_error() {
        let sandbox = create_sandbox("migrate");
        sandbox.enable_git();

        // create a new file at sandbox path to simulate a dirty repository
        fs::write(sandbox.path().join("new_file"), "new_file").unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["migrate", "from-package-json", "common"]);
        });

        assert
            .failure()
            .code(1)
            .stdout("")
            .stderr(contains("Commit or stash"));
    }

    #[test]
    fn converts_scripts() {
        let sandbox = create_sandbox("migrate");

        let assert = sandbox.run_moon(|cmd| {
            cmd.args([
                "migrate",
                "--skipTouchedFilesCheck",
                "from-package-json",
                "common",
            ]);
        });

        assert_snapshot!(fs::read_to_string(
            sandbox.path().join("package-json/common/package.json")
        )
        .unwrap());

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/common/moon.yml")).unwrap()
        );

        assert.success();
    }

    #[test]
    fn links_depends_on() {
        let sandbox = create_sandbox("migrate");

        let assert = sandbox.run_moon(|cmd| {
            cmd.args([
                "migrate",
                "--skipTouchedFilesCheck",
                "from-package-json",
                "deps",
            ]);
        });

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/deps/package.json")).unwrap()
        );

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/deps/moon.yml")).unwrap()
        );

        assert.success();
    }
}
