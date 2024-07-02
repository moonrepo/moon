use moon_common::Id;
use moon_config::{PartialVcsConfig, PartialWorkspaceConfig, PartialWorkspaceProjects};
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs};
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

    #[test]
    fn removes_codeowners_file() {
        let (workspace_config, _, _) = get_cases_fixture_configs();
        let sandbox = create_sandbox_with_config("cases", Some(workspace_config), None, None);

        assert!(!sandbox.path().join(".github/CODEOWNERS").exists());

        sandbox
            .run_moon(|cmd| {
                cmd.arg("sync").arg("codeowners");
            })
            .success();

        assert!(sandbox.path().join(".github/CODEOWNERS").exists());

        sandbox
            .run_moon(|cmd| {
                cmd.arg("sync").arg("codeowners").arg("--clean");
            })
            .success();

        assert!(!sandbox.path().join(".github/CODEOWNERS").exists());
    }
}

mod sync_config_schemas {
    use super::*;

    #[test]
    fn creates_schemas_dir() {
        let (workspace_config, _, _) = get_cases_fixture_configs();
        let sandbox = create_sandbox_with_config("cases", Some(workspace_config), None, None);

        assert!(!sandbox.path().join(".moon/cache/schemas").exists());

        sandbox
            .run_moon(|cmd| {
                cmd.arg("sync").arg("config-schemas");
            })
            .success();

        assert!(sandbox.path().join(".moon/cache/schemas").exists());
    }
}

mod sync_hooks {
    use super::*;

    #[test]
    fn creates_hook_files() {
        let (mut workspace_config, _, _) = get_cases_fixture_configs();

        workspace_config.vcs = Some(PartialVcsConfig {
            hooks: Some(FxHashMap::from_iter([
                (
                    "pre-commit".into(),
                    vec!["moon run :lint".into(), "some-command".into()],
                ),
                ("post-push".into(), vec!["moon check --all".into()]),
            ])),
            ..Default::default()
        });

        let sandbox = create_sandbox_with_config("cases", Some(workspace_config), None, None);
        sandbox.enable_git();

        let hooks_dir = sandbox.path().join(".moon/hooks");

        assert!(!hooks_dir.exists());

        sandbox
            .run_moon(|cmd| {
                cmd.arg("sync").arg("hooks");
            })
            .success();

        assert!(hooks_dir.exists());

        if cfg!(windows) {
            assert!(hooks_dir.join("pre-commit.ps1").exists());
            assert!(hooks_dir.join("post-push.ps1").exists());
        } else {
            assert!(hooks_dir.join("pre-commit.sh").exists());
            assert!(hooks_dir.join("post-push.sh").exists());
        }
    }

    #[test]
    fn removes_hook_files() {
        let (mut workspace_config, _, _) = get_cases_fixture_configs();

        workspace_config.vcs = Some(PartialVcsConfig {
            hooks: Some(FxHashMap::from_iter([
                (
                    "pre-commit".into(),
                    vec!["moon run :lint".into(), "some-command".into()],
                ),
                ("post-push".into(), vec!["moon check --all".into()]),
            ])),
            ..Default::default()
        });

        let sandbox = create_sandbox_with_config("cases", Some(workspace_config), None, None);
        sandbox.enable_git();

        let hooks_dir = sandbox.path().join(".moon/hooks");

        assert!(!hooks_dir.exists());

        sandbox
            .run_moon(|cmd| {
                cmd.arg("sync").arg("hooks");
            })
            .success();

        assert!(hooks_dir.exists());

        sandbox
            .run_moon(|cmd| {
                cmd.arg("sync").arg("hooks").arg("--clean");
            })
            .success();

        assert!(!hooks_dir.exists());
    }
}

mod sync_projects {
    use super::*;

    #[test]
    fn syncs_all_projects() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
                (Id::raw("a"), "a".to_owned()),
                (Id::raw("b"), "b".to_owned()),
                (Id::raw("c"), "c".to_owned()),
                (Id::raw("d"), "d".to_owned()),
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

        assert.success();
    }

    #[test]
    fn runs_legacy_sync_command() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
                (Id::raw("a"), "a".to_owned()),
                (Id::raw("b"), "b".to_owned()),
                (Id::raw("c"), "c".to_owned()),
                (Id::raw("d"), "d".to_owned()),
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

        assert.success();
    }
}
