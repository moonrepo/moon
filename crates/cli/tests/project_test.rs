mod utils;

use moon_test_utils::create_moon_sandbox;
use moon_test_utils::predicates::prelude::*;
use starbase_sandbox::assert_snapshot;
use starbase_utils::json::serde_json;
use utils::create_projects_sandbox;

mod project {
    use super::*;

    #[test]
    fn invalid_project_id() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("project").arg("=");
            })
            .failure()
            .code(2)
            .stderr(predicate::str::contains("invalid value '=' for '[ID]'"));
    }

    #[test]
    fn unknown_project() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("project").arg("unknown");
            })
            .failure()
            .code(1)
            .stderr(predicate::str::contains(
                "No project has been configured with the identifier or alias unknown",
            ));
    }

    #[test]
    fn empty_config() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("empty-config");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn no_config() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("no-config");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn basic_config() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("basic");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn advanced_config() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("advanced");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn depends_on() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("dep-foo");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn with_tasks() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("tasks");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn root_level() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("root");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn includes_metadata() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("metadata");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("project").arg("basic").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("{"));
    }
}

mod project_dependencies_roots {
    use super::*;

    #[test]
    fn lists_the_workspace_root_of_a_member() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("b");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn lists_a_package_root() {
        let sandbox = create_moon_sandbox("layout");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("project").arg("standalone");
            })
            .success()
            .stdout(predicate::str::contains(
                "Toolchain:\n    - tc-tier2 @ standalone (package root)",
            ));
    }

    #[test]
    fn doesnt_list_roots_that_its_not_a_member_of() {
        let sandbox = create_moon_sandbox("layout");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("project").arg("tools");
            })
            .success()
            .stdout(predicate::str::contains("Toolchains").not());
    }

    #[test]
    fn includes_roots_in_json() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("b").arg("--json");
        });

        let json: serde_json::Value = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(json["id"], "b");
        assert_eq!(
            json["dependenciesRoots"],
            serde_json::json!([
                { "toolchain": "tc-tier2", "root": "", "role": "workspaceMember" }
            ])
        );
    }

    #[test]
    fn includes_no_roots_in_json_when_not_a_member() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project").arg("tools").arg("--json");
        });

        let json: serde_json::Value = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(json["dependenciesRoots"], serde_json::json!([]));
    }
}
