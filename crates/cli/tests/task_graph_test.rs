mod utils;

use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use utils::create_projects_sandbox;

mod task_graph {
    use super::*;

    #[test]
    fn no_tasks() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn many_tasks() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_task_with_dependencies() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task-graph").arg("metadata:test").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_task_with_dependents() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task-graph")
                .arg("metadata:build")
                .arg("--dot")
                .arg("--dependents");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_task_no_dependencies() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task-graph").arg("advanced:build").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("task-graph").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("{"));
    }
}
