mod utils;

use moon_test_utils::{create_empty_moon_sandbox, predicates::prelude::*};
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
                .arg("--dependents")
                .arg("deep");
        });

        assert_snapshot!(assert.output());
    }

    fn create_dependents_chain(sandbox: &moon_test_utils::MoonSandbox) {
        sandbox.create_file(
            "chain/moon.yml",
            r#"tasks:
  base:
    command: 'noop'
  mid:
    command: 'noop'
    deps: ['~:base']
  top:
    command: 'noop'
    deps: ['~:mid']
"#,
        );
    }

    #[test]
    fn single_task_with_direct_dependents() {
        let sandbox = create_projects_sandbox();

        create_dependents_chain(&sandbox);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task-graph")
                .arg("chain:base")
                .arg("--dot")
                .arg("--dependents")
                .arg("direct");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#"label="chain:base""#).eval(&output));
        assert!(predicate::str::contains(r#"label="chain:mid""#).eval(&output));
        assert!(!predicate::str::contains(r#"label="chain:top""#).eval(&output));

        assert.success();
    }

    #[test]
    fn single_task_with_deep_dependents() {
        let sandbox = create_projects_sandbox();

        create_dependents_chain(&sandbox);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("task-graph")
                .arg("chain:base")
                .arg("--dot")
                .arg("--dependents")
                .arg("deep");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#"label="chain:base""#).eval(&output));
        assert!(predicate::str::contains(r#"label="chain:mid""#).eval(&output));
        assert!(predicate::str::contains(r#"label="chain:top""#).eval(&output));

        assert.success();
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
