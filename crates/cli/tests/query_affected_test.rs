mod utils;

use moon_affected::{Affected, AffectedProjectState, AffectedTaskState};
use moon_task::Target;
use rustc_hash::FxHashSet;
use starbase_utils::json::serde_json;
use utils::create_query_sandbox;

mod query_affected {
    use super::*;

    #[test]
    fn nothing_by_default() {
        let sandbox = create_query_sandbox();

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
        sandbox.create_file("basic/file.txt", "");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("affected");
        });

        let mut affected: Affected = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert!(!affected.projects.contains_key("advanced"));
        assert_eq!(
            affected.projects.remove("basic").unwrap(),
            AffectedProjectState {
                files: FxHashSet::from_iter(["basic/file.txt".into()]),
                tasks: FxHashSet::from_iter([Target::parse("basic:dev").unwrap()]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn includes_task_for_input() {
        let sandbox = create_query_sandbox();
        let target = Target::parse("tasks:test").unwrap();

        // Run first without any file
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("affected");
        });

        let affected: Affected = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert!(!affected.tasks.contains_key(&target));

        // Run again with file
        sandbox.create_file("tasks/tests/file.txt", "");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("affected");
        });

        let mut affected: Affected = serde_json::from_str(assert.stdout().trim()).unwrap();

        assert_eq!(
            affected.tasks.remove(&target).unwrap(),
            AffectedTaskState {
                files: FxHashSet::from_iter(["tasks/tests/file.txt".into()]),
                ..Default::default()
            }
        );
    }
}
