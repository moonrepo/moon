mod utils;

use moon_test_utils::{create_empty_moon_sandbox, predicates::prelude::*};
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
                .arg("--dependents")
                .arg("deep");
        });

        assert_snapshot!(assert.output());
    }

    // dep-bar <- dep-foo <- dep-qux
    fn create_dependents_chain(sandbox: &moon_test_utils::MoonSandbox) {
        sandbox.create_file("dep-qux/moon.yml", "dependsOn: ['dep-foo']\n");
    }

    #[test]
    fn single_project_with_direct_dependents() {
        let sandbox = create_projects_sandbox();

        create_dependents_chain(&sandbox);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph")
                .arg("dep-bar")
                .arg("--dot")
                .arg("--dependents")
                .arg("direct");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#"label="dep-bar""#).eval(&output));
        assert!(predicate::str::contains(r#"label="dep-foo""#).eval(&output));
        assert!(!predicate::str::contains(r#"label="dep-qux""#).eval(&output));

        assert.success();
    }

    #[test]
    fn single_project_with_deep_dependents() {
        let sandbox = create_projects_sandbox();

        create_dependents_chain(&sandbox);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("project-graph")
                .arg("dep-bar")
                .arg("--dot")
                .arg("--dependents")
                .arg("deep");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#"label="dep-bar""#).eval(&output));
        assert!(predicate::str::contains(r#"label="dep-foo""#).eval(&output));
        assert!(predicate::str::contains(r#"label="dep-qux""#).eval(&output));

        assert.success();
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
