mod utils;

use moon_test_utils2::predicates::prelude::*;
use starbase_sandbox::assert_snapshot;
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
