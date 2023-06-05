use moon_config::{PartialWorkspaceConfig, WorkspaceProjects};
use moon_test_utils::{create_sandbox_with_config, predicates::prelude::*};
use rustc_hash::FxHashMap;

#[test]
fn syncs_all_projects() {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("a".into(), "a".to_owned()),
            ("b".into(), "b".to_owned()),
            ("c".into(), "c".to_owned()),
            ("d".into(), "d".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/dependencies",
        Some(workspace_config),
        None,
        None,
    );

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("sync");
    });

    let output = assert.output();

    // Output is non-deterministic
    assert!(predicate::str::contains("SyncSystemProject(a)").eval(&output));
    assert!(predicate::str::contains("SyncSystemProject(b)").eval(&output));
    assert!(predicate::str::contains("SyncSystemProject(c)").eval(&output));
    assert!(predicate::str::contains("SyncSystemProject(d)").eval(&output));

    assert.success();
}
