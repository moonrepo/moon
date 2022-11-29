use moon_config::{WorkspaceConfig, WorkspaceProjects};
use moon_test_utils::{assert_snapshot, create_sandbox_with_config};
use rustc_hash::FxHashMap;

#[test]
fn syncs_all_projects() {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("a".to_owned(), "a".to_owned()),
            ("b".to_owned(), "b".to_owned()),
            ("c".to_owned(), "c".to_owned()),
            ("d".to_owned(), "d".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/dependencies",
        Some(&workspace_config),
        None,
        None,
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("sync");
    });

    assert_snapshot!(assert.output());

    assert.success();
}
