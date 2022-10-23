use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, create_sandbox, create_sandbox_with_git};
use predicates::str::contains;
use std::fs;

mod from_package_json {
    use super::*;

    #[test]
    fn dirty_repository_raises_an_error() {
        let fixture = create_sandbox_with_git("migrate");
        // create a new file at fixture path to simulate a dirty repository
        fs::write(fixture.path().join("new_file"), "new_file").unwrap();
        let assert = create_moon_command(fixture.path())
            .args(["migrate", "from-package-json", "common"])
            .assert();
        assert
            .failure()
            .code(1)
            .stdout("")
            .stderr(contains("Commit or stash"));
    }

    #[test]
    fn converts_scripts() {
        let fixture = create_sandbox("migrate");

        let assert = create_moon_command(fixture.path())
            .args([
                "migrate",
                "--skipTouchedFilesCheck",
                "from-package-json",
                "common",
            ])
            .assert();

        assert_snapshot!(fs::read_to_string(
            fixture.path().join("package-json/common/package.json")
        )
        .unwrap());

        assert_snapshot!(
            fs::read_to_string(fixture.path().join("package-json/common/moon.yml")).unwrap()
        );

        assert.success();
    }

    #[test]
    fn links_depends_on() {
        let fixture = create_sandbox("migrate");

        let assert = create_moon_command(fixture.path())
            .args([
                "migrate",
                "--skipTouchedFilesCheck",
                "from-package-json",
                "deps",
            ])
            .assert();

        assert_snapshot!(
            fs::read_to_string(fixture.path().join("package-json/deps/package.json")).unwrap()
        );

        assert_snapshot!(
            fs::read_to_string(fixture.path().join("package-json/deps/moon.yml")).unwrap()
        );

        assert.success();
    }
}
