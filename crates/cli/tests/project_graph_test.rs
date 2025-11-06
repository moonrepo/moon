mod utils;

use moon_test_utils2::{create_empty_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use utils::create_projects_sandbox;

mod project_graph {
    use super::*;

    #[test]
    fn no_projects() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn many_projects() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_project_with_dependencies() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("dep-foo").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_project_with_dependents() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph")
                .arg("dep-bar")
                .arg("--dot")
                .arg("--dependents");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn single_project_no_dependencies() {
        let sandbox = create_projects_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph").arg("dep-baz").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_projects_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("project-graph").arg("--json");
            })
            .success()
            .stdout(predicate::str::starts_with("{"));
    }
}
