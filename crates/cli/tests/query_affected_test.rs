mod utils;

use moon_affected::{Affected, AffectedProjectState, AffectedTaskState};
use moon_common::is_ci;
use moon_task::Target;
use rustc_hash::FxHashSet;
use starbase_utils::json::serde_json;
use utils::{change_branch, change_files, create_query_sandbox};

mod query_affected {
    use super::*;

    #[test]
    fn nothing_by_default() {
        let sandbox = create_query_sandbox();

        change_branch(&sandbox, "branch");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("query").arg("affected");
            })
            .success()
            .stdout("{}\n");
    }

    #[test]
    fn includes_project_for_file() {
        let sandbox = create_query_sandbox();

        change_files(&sandbox, ["basic/file.txt"]);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("affected");
        });

        let mut affected: Affected = serde_json::from_str(assert.stdout().trim()).unwrap();

        let expected_tasks = if is_ci() {
            FxHashSet::default()
        } else {
            FxHashSet::from_iter([Target::parse("basic:dev").unwrap()])
        };

        assert!(!affected.projects.contains_key("advanced"));
        assert_eq!(
            affected.projects.remove("basic").unwrap(),
            AffectedProjectState {
                files: FxHashSet::from_iter(["basic/file.txt".into()]),
                tasks: expected_tasks,
                ..Default::default()
            }
        );
    }

    #[test]
    fn includes_task_for_input() {
        let sandbox = create_query_sandbox();

        change_files(&sandbox, ["tasks/tests/file.txt"]);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("affected");
        });

        let mut affected: Affected = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(
            affected
                .tasks
                .remove(&Target::parse("tasks:test").unwrap())
                .unwrap(),
            AffectedTaskState {
                files: FxHashSet::from_iter(["tasks/tests/file.txt".into()]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn respects_run_in_ci() {
        let sandbox = create_query_sandbox();

        sandbox.create_file(
            "tasks/moon.yml",
            r#"tasks:
  default:
    command: echo default
    inputs: ['file.txt']
  ci-only:
    command: echo only
    inputs: ['file.txt']
    options:
      runInCI: only
  ci-disabled:
    command: echo never
    inputs: ['file.txt']
    options:
      runInCI: false
"#,
        );

        change_files(&sandbox, ["tasks/file.txt"]);

        // When in CI
        let assert_ci = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("affected").env("CI", "true");
        });

        let affected_ci: Affected = serde_json::from_str(assert_ci.stdout().trim()).unwrap();
        assert!(
            affected_ci
                .tasks
                .contains_key(&Target::parse("tasks:default").unwrap())
        );
        assert!(
            affected_ci
                .tasks
                .contains_key(&Target::parse("tasks:ci-only").unwrap())
        );
        assert!(
            !affected_ci
                .tasks
                .contains_key(&Target::parse("tasks:ci-disabled").unwrap())
        );

        // When NOT in CI (local mode checks uncommitted changes)
        sandbox.create_file("tasks/file.txt", "uncommitted change");

        let assert_local = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("affected")
                .env_remove("CI")
                .env_remove("CI_NAME")
                .env_remove("AZURE_PIPELINES")
                .env_remove("GITHUB_ACTIONS");
        });

        let affected_local: Affected = serde_json::from_str(assert_local.stdout().trim()).unwrap();
        assert!(
            affected_local
                .tasks
                .contains_key(&Target::parse("tasks:default").unwrap())
        );
        assert!(
            !affected_local
                .tasks
                .contains_key(&Target::parse("tasks:ci-only").unwrap())
        );
        assert!(
            affected_local
                .tasks
                .contains_key(&Target::parse("tasks:ci-disabled").unwrap())
        );
    }
}
