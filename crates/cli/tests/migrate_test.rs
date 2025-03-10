use moon_config::{PartialWorkspaceConfig, PartialWorkspaceProjects};
use moon_test_utils::{
    Sandbox, assert_snapshot, create_sandbox_with_config, get_default_toolchain,
    predicates::str::contains,
};
use starbase_utils::string_vec;
use std::fs;

fn migrate_sandbox() -> Sandbox {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Globs(string_vec![
            "package-json/*",
        ])),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    create_sandbox_with_config(
        "migrate",
        Some(workspace_config),
        Some(toolchain_config),
        None,
    )
}

mod from_package_json {
    use super::*;

    #[test]
    fn dirty_repository_raises_an_error() {
        let sandbox = migrate_sandbox();
        sandbox.enable_git();

        // create a new file at sandbox path to simulate a dirty repository
        sandbox.create_file("new_file", "new_file");

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
        let sandbox = migrate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.args([
                "migrate",
                "--skipTouchedFilesCheck",
                "from-package-json",
                "common",
            ]);
        });

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/common/package.json")).unwrap()
        );

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/common/moon.yml")).unwrap()
        );

        assert.success();
    }

    #[test]
    fn links_depends_on() {
        let sandbox = migrate_sandbox();

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
