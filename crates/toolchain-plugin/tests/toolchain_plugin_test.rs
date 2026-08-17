use moon_common::Id;
use moon_pdk_api::{DefineRequirementsInput, ExtendTaskCommandInput, LocateDependenciesRootInput};
use moon_test_utils::WorkspaceMocker;
use moon_toolchain::{DependenciesWorkspace, DependenciesWorkspaceRole};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::fs;

fn create_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_test_toolchains();

    (sandbox, mocker)
}

// A working directory named `in` triggers the dependencies workspace
// branch within the `tc-tier2` test plugin
fn create_deps_workspace() -> (Sandbox, WorkspaceMocker) {
    let sandbox = create_empty_sandbox();
    let root = sandbox.path().join("in");

    fs::create_dir_all(&root).unwrap();

    let mocker = WorkspaceMocker::new(&root).with_test_toolchains();

    (sandbox, mocker)
}

mod toolchain_plugin {
    use super::*;

    mod guarded_funcs {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn define_toolchain_config_returns_none_when_func_missing() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier1").await.unwrap();

            assert!(toolchain.define_toolchain_config().await.unwrap().is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn define_requirements_returns_default_when_func_missing() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier1").await.unwrap();

            let output = toolchain
                .define_requirements(DefineRequirementsInput {
                    context: registry.create_context(),
                    ..Default::default()
                })
                .await
                .unwrap();

            assert!(output.requires.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extend_task_command_returns_default_when_func_missing() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier1").await.unwrap();

            let output = toolchain
                .extend_task_command(ExtendTaskCommandInput {
                    context: registry.create_context(),
                    ..Default::default()
                })
                .await
                .unwrap();

            assert!(output.command.is_none());
            assert!(output.paths.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locate_dependencies_root_returns_none_when_func_missing() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier1").await.unwrap();
            let context = registry.create_context();

            let output = toolchain
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: context.working_dir.clone(),
                    context,
                    ..Default::default()
                })
                .await
                .unwrap();

            assert!(output.is_none());
        }
    }

    mod define_requirements {
        use super::*;
        use starbase_utils::json::json;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_requirements_for_setup_toolchain() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier2-reqs").await.unwrap();

            let output = toolchain
                .define_requirements(DefineRequirementsInput {
                    context: registry.create_context(),
                    ..Default::default()
                })
                .await
                .unwrap();

            assert_eq!(output.requires, vec!["tc-tier3".to_string()]);
            assert!(output.for_setup_toolchain);
            assert!(!output.for_setup_environment);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn flags_requirements_for_setup_environment_when_opted_in() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier2-reqs").await.unwrap();

            let output = toolchain
                .define_requirements(DefineRequirementsInput {
                    context: registry.create_context(),
                    toolchain_config: json!({
                        "testRequiresForEnvironment": true,
                    }),
                })
                .await
                .unwrap();

            assert_eq!(output.requires, vec!["tc-tier3".to_string()]);
            assert!(output.for_setup_toolchain);
            assert!(output.for_setup_environment);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_requirements_for_setup_environment_from_config() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier2-setup-env").await.unwrap();

            let output = toolchain
                .define_requirements(DefineRequirementsInput {
                    context: registry.create_context(),
                    toolchain_config: json!({
                        "testEnvRequirements": ["tc-tier3"],
                    }),
                })
                .await
                .unwrap();

            assert_eq!(output.requires, vec!["tc-tier3".to_string()]);
            assert!(!output.for_setup_toolchain);
            assert!(output.for_setup_environment);
        }
    }

    mod locate_dependencies_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn maps_output_into_a_dependencies_workspace() {
            let (_sandbox, ws) = create_deps_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier2").await.unwrap();
            let context = registry.create_context();

            let workspace = toolchain
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: context.working_dir.clone(),
                    context,
                    ..Default::default()
                })
                .await
                .unwrap()
                .unwrap();

            // The virtual `/workspace` root is converted back to a real path
            assert_eq!(workspace.root, ws.workspace_root);
            assert_eq!(workspace.members, Some(vec!["in".to_string()]));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn unsets_members_when_no_workspace_detected() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier2").await.unwrap();
            let context = registry.create_context();

            let workspace = toolchain
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: context.working_dir.clone(),
                    context,
                    ..Default::default()
                })
                .await
                .unwrap()
                .unwrap();

            // Outside a dependencies workspace, `tc-tier2` returns the
            // working directory as the root, and no members
            assert_eq!(workspace.root, ws.working_dir);
            assert_eq!(workspace.members, None);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn many_filters_out_toolchains_without_a_root() {
            let (_sandbox, ws) = create_deps_workspace();
            let registry = ws.mock_toolchain_registry();

            let results = registry
                .locate_dependencies_root_many(
                    vec![&Id::raw("tc-tier1"), &Id::raw("tc-tier2")],
                    |_toolchain| LocateDependenciesRootInput {
                        starting_dir: registry.create_context().working_dir,
                        context: registry.create_context(),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            // tier1 doesn't support the func, so only tier2 remains
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].id, Id::raw("tc-tier2"));
            assert_eq!(results[0].output.members, Some(vec!["in".to_string()]));
        }
    }

    mod in_dependencies_workspace {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn matches_the_root_and_member_globs() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier1").await.unwrap();

            let workspace = DependenciesWorkspace {
                root: ws.workspace_root.clone(),
                members: Some(vec!["packages/*".into()]),
            };

            // Root is always within the workspace
            assert_eq!(
                toolchain
                    .in_dependencies_workspace(&workspace, &ws.workspace_root)
                    .unwrap(),
                Some(DependenciesWorkspaceRole::WorkspaceRoot)
            );

            // Matches a member glob
            assert_eq!(
                toolchain
                    .in_dependencies_workspace(&workspace, &ws.workspace_root.join("packages/foo"))
                    .unwrap(),
                Some(DependenciesWorkspaceRole::WorkspaceMember)
            );

            // Doesn't match a member glob
            assert_eq!(
                toolchain
                    .in_dependencies_workspace(&workspace, &ws.workspace_root.join("apps/foo"))
                    .unwrap(),
                None
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn empty_members_only_matches_the_root() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier1").await.unwrap();

            let workspace = DependenciesWorkspace {
                root: ws.workspace_root.clone(),
                members: Some(vec![]),
            };

            assert_eq!(
                toolchain
                    .in_dependencies_workspace(&workspace, &ws.workspace_root)
                    .unwrap(),
                Some(DependenciesWorkspaceRole::WorkspaceRoot)
            );

            // A workspace with no members has nothing to match against
            assert_eq!(
                toolchain
                    .in_dependencies_workspace(&workspace, &ws.workspace_root.join("packages/foo"))
                    .unwrap(),
                None
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_members_means_a_single_root_package() {
            let (_sandbox, ws) = create_workspace();
            let registry = ws.mock_toolchain_registry();
            let toolchain = registry.load("tc-tier1").await.unwrap();

            let workspace = DependenciesWorkspace {
                root: ws.workspace_root.clone(),
                members: None,
            };

            // The root is the package
            assert_eq!(
                toolchain
                    .in_dependencies_workspace(&workspace, &ws.workspace_root)
                    .unwrap(),
                Some(DependenciesWorkspaceRole::PackageRoot)
            );

            // And is the only package, so there's nothing else to match
            assert_eq!(
                toolchain
                    .in_dependencies_workspace(&workspace, &ws.workspace_root.join("apps/foo"))
                    .unwrap(),
                None
            );
        }
    }
}
