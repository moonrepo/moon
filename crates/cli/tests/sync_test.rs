use moon_config::{PartialWorkspaceConfig, WorkspaceProjects};
use moon_test_utils::{
    create_sandbox_with_config, get_cases_fixture_configs, predicates::prelude::*,
};
use rustc_hash::FxHashMap;

mod sync_codeowners {
    use super::*;

    #[test]
    fn creates_codeowners_file() {
        let (workspace_config, _, _) = get_cases_fixture_configs();
        let sandbox = create_sandbox_with_config("cases", Some(workspace_config), None, None);

        assert!(!sandbox.path().join(".github/CODEOWNERS").exists());

        sandbox
            .run_moon(|cmd| {
                cmd.arg("sync").arg("codeowners");
            })
            .success();

        assert!(sandbox.path().join(".github/CODEOWNERS").exists());
    }
}

mod sync_projects {
    use super::*;

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
            cmd.arg("sync").arg("projects");
        });

        let output = assert.output();

        // Output is non-deterministic
        assert!(predicate::str::contains("SyncSystemProject(a)").eval(&output));
        assert!(predicate::str::contains("SyncSystemProject(b)").eval(&output));
        assert!(predicate::str::contains("SyncSystemProject(c)").eval(&output));
        assert!(predicate::str::contains("SyncSystemProject(d)").eval(&output));

        assert.success();
    }

    #[test]
    fn runs_legacy_sync_command() {
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
            cmd.arg("sync"); // <-- this
        });

        let output = assert.output();

        // Output is non-deterministic
        assert!(predicate::str::contains("SyncSystemProject(a)").eval(&output));
        assert!(predicate::str::contains("SyncSystemProject(b)").eval(&output));
        assert!(predicate::str::contains("SyncSystemProject(c)").eval(&output));
        assert!(predicate::str::contains("SyncSystemProject(d)").eval(&output));

        assert.success();
    }
}
