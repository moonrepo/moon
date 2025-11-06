mod utils;

use moon_test_utils2::predicates::prelude::*;
use starbase_sandbox::assert_snapshot;
use utils::create_projects_sandbox;

mod task {
    use super::*;

    #[test]
    fn unknown_task() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("task").arg("tasks:unknown");
            })
            .failure()
            .code(1)
            .stderr(predicate::str::contains(
                "Unknown task unknown for project tasks.",
            ));
    }

    #[test]
    fn shows_inputs() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task").arg("tasks:test");
        });

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn shows_outputs() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task").arg("tasks:lint");
        });

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn can_show_internal() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task").arg("tasks:internal");
        });

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("task").arg("tasks:lint").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("{"));
    }
}
