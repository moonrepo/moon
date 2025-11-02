mod utils;

use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use utils::create_projects_sandbox;

mod tasks {
    use super::*;

    #[test]
    fn no_tasks() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("tasks");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn no_tasks_for_a_project() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("tasks").arg("no-config");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn many_tasks() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("tasks");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_focus_for_a_project() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("tasks").arg("tasks");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("tasks").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("["));
    }
}
