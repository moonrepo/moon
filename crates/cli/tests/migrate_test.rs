use insta::assert_snapshot;
use moon_utils::test::{create_moon_command_in, create_sandbox};
use std::fs;

mod from_package_json {
    use super::*;

    #[test]
    fn converts_scripts() {
        let fixture = create_sandbox("migrate");

        let assert = create_moon_command_in(fixture.path())
            .args(["migrate", "from-package-json", "common"])
            .assert();

        assert_snapshot!(fs::read_to_string(
            fixture.path().join("package-json/common/package.json")
        )
        .unwrap());

        assert_snapshot!(fs::read_to_string(
            fixture.path().join("package-json/common/project.yml")
        )
        .unwrap());

        assert.success();
    }

    #[test]
    fn links_depends_on() {
        let fixture = create_sandbox("migrate");

        let assert = create_moon_command_in(fixture.path())
            .args(["migrate", "from-package-json", "deps"])
            .assert();

        assert_snapshot!(
            fs::read_to_string(fixture.path().join("package-json/deps/package.json")).unwrap()
        );

        assert_snapshot!(
            fs::read_to_string(fixture.path().join("package-json/deps/project.yml")).unwrap()
        );

        assert.success();
    }
}
