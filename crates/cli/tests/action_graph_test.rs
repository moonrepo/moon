mod utils;

use moon_test_utils2::{create_moon_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use utils::create_tasks_sandbox;

mod action_graph {
    use super::*;

    #[test]
    fn errors_for_cycle() {
        let sandbox = create_moon_sandbox("tasks-cycle");
        sandbox.with_default_projects();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("action-graph").arg("--dot");
        });

        assert
            .failure()
            .stderr(predicate::str::contains("would introduce a cycle"));
    }

    #[test]
    fn all_by_default() {
        let sandbox = create_tasks_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("action-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn focused_by_target() {
        let sandbox = create_tasks_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("ag").arg("--dot").arg("basic:lint");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn includes_dependencies_when_focused() {
        let sandbox = create_tasks_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("action-graph").arg("--dot").arg("chain:e");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn includes_dependents_when_focused() {
        let sandbox = create_tasks_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("action-graph")
                .arg("--dot")
                .arg("--dependents")
                .arg("basic:build");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn can_output_json() {
        let sandbox = create_tasks_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("action-graph").arg("--json").arg("basic:lint");
            })
            .success()
            .stdout(predicate::str::starts_with("{"));
    }

    mod aliases {
        use super::*;

        #[test]
        fn can_focus_using_an_alias() {
            let sandbox = create_tasks_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("action-graph").arg("--dot").arg("@tasks/node:b");
            });

            assert_snapshot!(assert.output());
        }

        #[test]
        fn resolves_aliases_in_task_deps() {
            let sandbox = create_tasks_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("action-graph").arg("--dot").arg("@tasks/node:a");
            });

            assert_snapshot!(assert.output());
        }
    }
}
