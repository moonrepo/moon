mod utils;

use moon_action::*;
use moon_action_context::TargetState;
use moon_action_graph::{ActionGraph, ActionGraphBuilderOptions, RunRequirements};
use moon_affected::{AffectedBy, DownstreamScope, UpstreamScope};
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{
    EnvMap, PROTO_CLI_VERSION, PipelineActionSwitch, TaskDependencyConfig, TaskDependencyType,
    TaskOptionRunInCI, UnresolvedVersionSpec, Version, VersionSpec,
};
use moon_exec_plan::{ExecutionPlan, GraphBlock, TargetsBlock};
use moon_graph_utils::*;
use moon_task::{Target, TargetLocator, Task, TaskFileInput};
use moon_toolchain::ToolchainSpec;
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{assert_snapshot, create_sandbox};
use utils::ActionGraphContainer;

fn create_task(project: &str, id: &str) -> Task {
    Task {
        id: Id::raw(id),
        target: Target::new(project, id).unwrap(),
        toolchains: vec![Id::raw("node")],
        ..Task::default()
    }
}

fn create_proto_version() -> VersionSpec {
    VersionSpec::parse(PROTO_CLI_VERSION).unwrap()
}

fn create_unresolved_version(version: Version) -> UnresolvedVersionSpec {
    UnresolvedVersionSpec::Version(version)
}

fn create_node_spec() -> ToolchainSpec {
    ToolchainSpec::new(
        Id::raw("node"),
        UnresolvedVersionSpec::parse("20.0.0").unwrap(),
    )
}

fn create_tier_spec(tier: u8) -> ToolchainSpec {
    create_tier_spec_with_name(format!("tc-tier{tier}"))
}

fn create_tier_spec_with_name(id: impl AsRef<str>) -> ToolchainSpec {
    ToolchainSpec::new(
        Id::raw(id.as_ref()),
        create_unresolved_version(Version::new(1, 2, 3)),
    )
}

fn topo(graph: ActionGraph) -> Vec<ActionNode> {
    let mut nodes = vec![];

    for index in graph.sort_topological().unwrap() {
        nodes.push(graph.get_node_from_index(&index).unwrap().to_owned());
    }

    nodes
}

fn extract_run_task_targets(graph: ActionGraph) -> Vec<String> {
    let mut targets = topo(graph)
        .into_iter()
        .filter_map(|node| match node {
            ActionNode::RunTask(inner) => Some(inner.target.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    targets.sort();
    targets
}

mod action_graph_builder {
    use super::*;

    mod install_deps {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(1);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_if_tier2() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_toolchain_if_tier3() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(3);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: spec.clone()
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple_toolchain_versions() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec1 = create_tier_spec(3);

            let mut spec2 = create_tier_spec(3);
            spec2.req = Some(create_unresolved_version(Version::new(4, 5, 6)));

            let project = wg.get_project("bar").unwrap();
            builder
                .install_dependencies(&spec1, &project)
                .await
                .unwrap();
            builder
                .install_dependencies(&spec2, &project)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: spec1.clone()
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: spec2 }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec1.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple_toolchain_versions_using_overrides() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let project = wg.get_project("bar").unwrap();
            builder
                .install_dependencies(
                    &builder
                        .get_project_spec(&Id::raw("rust"), &project)
                        .unwrap(),
                    &project,
                )
                .await
                .unwrap();

            let project = wg.get_project("qux").unwrap();
            builder
                .install_dependencies(
                    &builder
                        .get_project_spec(&Id::raw("rust"), &project)
                        .unwrap(),
                    &project,
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("rust"),
                            UnresolvedVersionSpec::parse("1.70.0").unwrap()
                        )
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("rust"),
                            UnresolvedVersionSpec::parse("1.90.0").unwrap()
                        )
                    }),
                    // No install dependencies because `Cargo.toml`
                    // is not setup in the fixture!
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_env_chain_if_defined() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_env_chain_with_toolchain_requirements() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-setup-env")) {
                    inner.config.insert(
                        "testEnvRequirements".into(),
                        serde_json::json!(["tc-tier3"]),
                    );
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled_in_config() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let spec = create_tier_spec(2);

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&spec.id) {
                    inner.install_dependencies = false;
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: PipelineActionSwitch::Only(vec![Id::raw("rust")]),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: PipelineActionSwitch::Only(vec![Id::raw("tc-tier2")]),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_project_if_in_project_root() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("isolated"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("isolated").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: Some(Id::raw("isolated")),
                        root: WorkspaceRelativePathBuf::from("isolated"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_not_in_deps_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("out"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            // The project is not a member of the dependencies workspace, so it
            // owns no environment. It only wants the toolchain binaries on `PATH`,
            // and provisioning one here would clobber the workspace's environment,
            // since the package manager resolves upwards to the same root
            let project = wg.get_project("out").unwrap();
            let index = builder.install_dependencies(&spec, &project).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_not_the_root_package() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            // There's no workspace, so the located root is the only package,
            // and this project isn't it
            let project = wg.get_project("bar").unwrap();
            let index = builder.install_dependencies(&spec, &project).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_toolchain_only_if_not_in_deps_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("out"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // Unlike tier 2, tier 3 can still setup the toolchain itself,
            // so that its binaries are available to the project's tasks
            let spec = create_tier_spec(3);

            let project = wg.get_project("out").unwrap();
            let index = builder.install_dependencies(&spec, &project).await.unwrap();

            assert!(index.is_some());

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: spec }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_in_deps_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("in"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("in").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_in_deps_workspace_if_root_level() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("in-root"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("root").unwrap();
            builder.install_dependencies(&spec, &project).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }
    }

    mod install_deps_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_if_deps_root_is_workspace_root() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let index = builder.install_dependencies_root(&spec).await.unwrap();

            assert!(index.is_some());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_toolchain_if_tier3() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(3);

            builder.install_dependencies_root(&spec).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: spec.clone()
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_env_chain_if_defined() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            builder.install_dependencies_root(&spec).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_members_if_in_deps_workspace() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("in"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            builder.install_dependencies_root(&spec).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn dedupes_with_project_based_flow() {
            let sandbox = create_sandbox("dep-workspace");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("in"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let project = wg.get_project("in").unwrap();
            let project_index = builder.install_dependencies(&spec, &project).await.unwrap();
            let root_index = builder.install_dependencies_root(&spec).await.unwrap();

            assert_eq!(project_index, root_index);

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_deps_root_isnt_workspace_root() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(2);

            let index = builder.install_dependencies_root(&spec).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(1);

            let index = builder.install_dependencies_root(&spec).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            builder.install_dependencies_root(&spec).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled_in_config() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let spec = create_tier_spec(2);

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&spec.id) {
                    inner.install_dependencies = false;
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            builder.install_dependencies_root(&spec).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        install_dependencies: PipelineActionSwitch::Only(vec![Id::raw("rust")]),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec(2);

            builder.install_dependencies_root(&spec).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }

    mod run_task {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone()))
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone()))
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_interactive() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let mut task = create_task("bar", "build");
            task.options.interactive = true;

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target);
                    node.interactive = true;
                    node
                })
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_interactive_from_requirement() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task(
                    &task,
                    &RunRequirements {
                        interactive: true,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target);
                    node.interactive = true;
                    node
                })
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_persistent() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let mut task = create_task("bar", "build");
            task.options.persistent = true;

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph).last().unwrap(),
                &ActionNode::run_task({
                    let mut node = RunTaskNode::new(task.target);
                    node.persistent = true;
                    node
                })
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn distinguishes_between_args() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            // Test collapsing
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["x".into(), "y".into(), "z".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone())),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["x".into(), "y".into(), "z".into()];
                        node
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn flattens_same_args() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn flattens_same_args_with_diff_enum() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn distinguishes_between_env() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            // Test collapsing
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone())),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.env = EnvMap::from_iter([("FOO".into(), Some("1".into()))]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.env = EnvMap::from_iter([("BAR".into(), Some("2".into()))]);
                        node
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn flattens_same_env() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.env = EnvMap::from_iter([("FOO".into(), Some("1".into()))]);
                        node
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn distinguishes_between_args_and_env() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            let task = create_task("bar", "build");

            // Test collapsing
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            // Separate nodes
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();
            builder
                .run_task_with_config(
                    &task,
                    &RunRequirements::default(),
                    &TaskDependencyConfig {
                        args: vec!["x".into(), "y".into(), "z".into()],
                        env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                        ..TaskDependencyConfig::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_node_spec(),
                    }),
                    ActionNode::run_task(RunTaskNode::new(task.target.clone())),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = EnvMap::from_iter([("FOO".into(), Some("1".into()))]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target.clone());
                        node.args = vec!["a".into(), "b".into(), "c".into()];
                        node.env = EnvMap::from_iter([("BAR".into(), Some("2".into()))]);
                        node
                    }),
                    ActionNode::run_task({
                        let mut node = RunTaskNode::new(task.target);
                        node.args = vec!["x".into(), "y".into(), "z".into()];
                        node.env = EnvMap::from_iter([("BAR".into(), Some("2".into()))]);
                        node
                    }),
                ]
            );
        }

        mod affected {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_graph_if_not_affected_by_changed_files() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let task = create_task("bar", "build");

                // Empty set works fine, just needs to be some
                let changed_files = FxHashSet::default();
                builder.set_changed_files(changed_files).unwrap();
                builder.set_affected().unwrap();

                builder
                    .run_task(&task, &RunRequirements::default())
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert!(!topo(graph).into_iter().any(|node| {
                    if let ActionNode::RunTask(inner) = &node {
                        inner.target.as_str() == "bar:build"
                    } else {
                        false
                    }
                }));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_affected_by_changed_files() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let file = WorkspaceRelativePathBuf::from("bar/file.js");

                let mut task = create_task("bar", "build");
                task.input_files
                    .insert(file.clone(), TaskFileInput::default());

                builder.mock_affected(FxHashSet::from_iter([file]), |affected| {
                    affected
                        .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                        .unwrap();
                });

                builder
                    .run_task(&task, &RunRequirements::default())
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn includes_deps_if_owning_task_is_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependents: DownstreamScope::Deep,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_eq!(
                    topo(graph),
                    vec![
                        ActionNode::sync_workspace(),
                        ActionNode::sync_project(SyncProjectNode {
                            project_id: Id::raw("deps-affected"),
                        }),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:d").unwrap(),
                        )),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:c").unwrap(),
                        )),
                        ActionNode::run_task(RunTaskNode::new(
                            Target::parse("deps-affected:b").unwrap(),
                        )),
                        // ActionNode::run_task(RunTaskNode::new(
                        //     Target::parse("deps-affected:a").unwrap(),
                        // )),
                    ]
                );
            }

            fn create_adverse_order_locators(targets: [&str; 6]) -> Vec<TargetLocator> {
                targets
                    .into_iter()
                    .map(|target| TargetLocator::Qualified(Target::parse(target).unwrap()))
                    .collect()
            }

            // The `affected-starve` fixture pins `asyncAffectedTracking: false`
            // to cover the synchronous tracker, whose marks would otherwise
            // depend on target insertion order: a task marked through another
            // task's relationship walk before its own visit never ran its own
            // checks and walks, starving transitive dependents of marks.
            #[tokio::test(flavor = "multi_thread")]
            async fn sync_includes_deep_dependents_regardless_of_target_order() {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("base/src.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                    },
                );

                // base:test first, so its walk marks base:build before its visit
                builder
                    .run_tasks(
                        create_adverse_order_locators([
                            "base:test",
                            "base:build",
                            "mid:build",
                            "mid:test",
                            "top:build",
                            "top:test",
                        ]),
                        RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Deep,
                            include_relations: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_eq!(
                    extract_run_task_targets(graph),
                    [
                        "base:build",
                        "base:test",
                        "mid:build",
                        "mid:test",
                        "top:build",
                        "top:test"
                    ]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn sync_includes_deep_dependents_when_base_task_ordered_last() {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("base/src.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                    },
                );

                builder
                    .run_tasks(
                        create_adverse_order_locators([
                            "mid:test",
                            "mid:build",
                            "top:test",
                            "top:build",
                            "base:test",
                            "base:build",
                        ]),
                        RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Deep,
                            include_relations: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_eq!(
                    extract_run_task_targets(graph),
                    [
                        "base:build",
                        "base:test",
                        "mid:build",
                        "mid:test",
                        "top:build",
                        "top:test"
                    ]
                );
            }

            // A superset change set must never schedule fewer tasks: the extra
            // changed file used to relation-mark the middle project's build
            // before its visit, dropping its test task entirely
            #[tokio::test(flavor = "multi_thread")]
            async fn sync_stays_monotonic_when_change_set_grows() {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.mock_affected(
                    FxHashSet::from_iter([
                        WorkspaceRelativePathBuf::from("base/src.txt"),
                        WorkspaceRelativePathBuf::from("top/src.txt"),
                    ]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                    },
                );

                builder
                    .run_tasks(
                        create_adverse_order_locators([
                            "top:test",
                            "top:build",
                            "mid:test",
                            "mid:build",
                            "base:test",
                            "base:build",
                        ]),
                        RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Deep,
                            include_relations: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_eq!(
                    extract_run_task_targets(graph),
                    [
                        "base:build",
                        "base:test",
                        "mid:build",
                        "mid:test",
                        "top:build",
                        "top:test"
                    ]
                );
            }

            // A target is marked through a relation only when the task on the
            // other side of that relation has been marked itself. When the only
            // affected task isn't one that was requested, tracking just the
            // requested targets leaves the relation unmarked, and the target is
            // dropped even though its dependency changed
            async fn run_only_downstream_target(
                async_tracking: bool,
                include_relations: bool,
            ) -> Vec<String> {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());

                container.mocker = container.mocker.update_workspace_config(|config| {
                    config.experiments.async_affected_tracking = async_tracking;
                });

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                // Only `base:build` is affected by this file
                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("base/src.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                    },
                );

                // While `top:build` is the only requested target
                builder
                    .run_tasks(
                        vec![TargetLocator::Qualified(
                            Target::parse("top:build").unwrap(),
                        )],
                        RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Deep,
                            include_relations,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                extract_run_task_targets(graph)
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn sync_marks_target_through_upstream_relations() {
                assert_eq!(
                    run_only_downstream_target(false, true).await,
                    // `top:test` depends on `top:build`, so deep dependents
                    // pulls it in, while the dependencies reached through
                    // `top:build` do not expand their own dependents
                    ["base:build", "mid:build", "top:build", "top:test"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn async_marks_target_through_upstream_relations() {
                assert_eq!(
                    run_only_downstream_target(true, true).await,
                    // `top:test` depends on `top:build`, so deep dependents
                    // pulls it in, while the dependencies reached through
                    // `top:build` do not expand their own dependents
                    ["base:build", "mid:build", "top:build", "top:test"]
                );
            }

            // But without relations, only the changed files themselves can mark
            // a task, so an unaffected target must stay out of the graph
            #[tokio::test(flavor = "multi_thread")]
            async fn sync_doesnt_mark_target_without_relations() {
                assert!(run_only_downstream_target(false, false).await.is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn async_doesnt_mark_target_without_relations() {
                assert!(run_only_downstream_target(true, false).await.is_empty());
            }
        }

        mod run_in_ci {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_ci_check_true() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(true);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_dependents_if_its_ci_false() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci3-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_dependents_if_both_are_ci_true() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci4-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }
        }

        mod dont_run_in_ci {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_graph_if_task_ci_skip() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Skip;

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(
                    context.get_target_states(),
                    FxHashMap::from_iter([(
                        Target::parse("bar:build").unwrap(),
                        TargetState::Passthrough
                    )])
                );

                assert!(topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_graph_if_ci_check_true() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(
                    context.get_target_states(),
                    FxHashMap::from_iter([(
                        Target::parse("bar:build").unwrap(),
                        TargetState::Passthrough
                    )])
                );

                assert!(topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_ci_check_false() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: false,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn graphs_if_ci_false() {
                let sandbox = create_sandbox("projects");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let mut task = create_task("bar", "build");
                task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: false,
                            ci_check: false,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, graph) = builder.build();

                assert_eq!(context.get_target_states(), FxHashMap::default());
                assert!(!topo(graph).is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_dependents_if_dependency_is_ci_false_and_not_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci2-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_dependents_if_both_are_ci_false() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci2-dependency").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            ci: true,
                            ci_check: true,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(
                expected = "Task ci:ci1-dependent cannot depend on task ci:ci1-dependency"
            )]
            async fn errors_if_dependency_is_ci_false_and_constraint_enabled() {
                let sandbox = create_sandbox("tasks-ci-mismatch");
                let mut container = ActionGraphContainer::new(sandbox.path());

                container
                    .create_builder(container.create_workspace_graph().await)
                    .await;
            }
        }

        mod dependencies {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn tracks_skipped_deps_for_none_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, _) = builder.build();

                assert_eq!(
                    context.ignored_dependencies,
                    FxHashMap::from_iter([(
                        Target::parse("deps:chain3").unwrap(),
                        FxHashSet::from_iter([Target::parse("deps:chain4").unwrap()])
                    )])
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::None, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Direct,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn tracks_skipped_transitive_deps_for_direct_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Direct,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (context, _) = builder.build();

                assert_eq!(
                    context.ignored_dependencies,
                    FxHashMap::from_iter([(
                        Target::parse("deps:chain4").unwrap(),
                        FxHashSet::from_iter([Target::parse("deps:chain5").unwrap()])
                    )])
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn clears_skipped_deps_when_task_is_revisited_in_scope() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let parent = wg.get_task_from_project("deps", "chain2").unwrap();
                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                let reqs = RunRequirements {
                    dependencies: UpstreamScope::Direct,
                    dependents: DownstreamScope::None,
                    ..RunRequirements::default()
                };

                builder.run_task(&parent, &reqs).await.unwrap();
                builder.run_task(&task, &reqs).await.unwrap();

                let (context, graph) = builder.build();

                assert!(
                    !context
                        .ignored_dependencies
                        .contains_key(&Target::parse("deps:chain3").unwrap())
                );
                assert_eq!(
                    context.ignored_dependencies,
                    FxHashMap::from_iter([(
                        Target::parse("deps:chain4").unwrap(),
                        FxHashSet::from_iter([Target::parse("deps:chain5").unwrap()])
                    )])
                );
                assert!(topo(graph).into_iter().any(|node| matches!(
                    node,
                    ActionNode::RunTask(inner) if inner.target == Target::parse("deps:chain4").unwrap()
                )));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::Direct, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Direct,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::Deep, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }
        }

        mod dependents {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_none_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "c").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/c.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::None, DownstreamScope::None);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Direct,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "c").unwrap();
                let task_b = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/c.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::None, DownstreamScope::Direct);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                        affected
                            .mark_task_affected(&task_b, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Direct,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "c").unwrap();
                let task_b = wg.get_task_from_project("deps-affected", "b").unwrap();
                let task_a = wg.get_task_from_project("deps-affected", "a").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/c.txt")]),
                    |affected| {
                        affected.set_scopes(UpstreamScope::None, DownstreamScope::Deep);
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                        affected
                            .mark_task_affected(&task_b, AffectedBy::AlwaysAffected)
                            .unwrap();
                        affected
                            .mark_task_affected(&task_a, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::None,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn expands_skipped_dependents_when_task_is_revisited_in_scope() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let parent = wg.get_task_from_project("deps", "parent1").unwrap();
                let task = wg.get_task_from_project("deps", "base").unwrap();

                // First insert the task as a dependency of another target,
                // with dependents out of scope
                builder
                    .run_task(
                        &parent,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::None,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                // Then run it as an explicit target with dependents in scope
                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Direct,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());

                assert!(topo(graph).into_iter().any(|node| matches!(
                    node,
                    ActionNode::RunTask(inner) if inner.target == Target::parse("deps:parent2").unwrap()
                )));
            }

            // The `dependent-scopes` fixture forms the following dependency
            // graph, where dependents of `c` exist outside of `a`'s upstream:
            //   a -> b -> c
            //   d -> e -> c
            #[tokio::test(flavor = "multi_thread")]
            async fn deep_doesnt_expand_dependents_of_dependencies() {
                let sandbox = create_sandbox("dependent-scopes");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("a", "task").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                // `a` has no dependents, so only its dependency chain runs;
                // `d` and `e` are dependents of the upstream `c`, not of `a`
                assert_eq!(
                    extract_run_task_targets(graph),
                    ["a:task", "b:task", "c:task"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn deep_expands_transitive_dependents_of_target() {
                let sandbox = create_sandbox("dependent-scopes");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("c", "task").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Deep,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                // Everything is downstream of `c`
                assert_eq!(
                    extract_run_task_targets(graph),
                    ["a:task", "b:task", "c:task", "d:task", "e:task"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn direct_expands_only_target_dependents() {
                let sandbox = create_sandbox("dependent-scopes");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("c", "task").unwrap();

                builder
                    .run_task(
                        &task,
                        &RunRequirements {
                            dependencies: UpstreamScope::Deep,
                            dependents: DownstreamScope::Direct,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_eq!(
                    extract_run_task_targets(graph),
                    ["b:task", "c:task", "e:task"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn deep_expands_dependents_when_dependency_is_also_a_target() {
                let sandbox = create_sandbox("dependent-scopes");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let reqs = RunRequirements {
                    dependencies: UpstreamScope::Deep,
                    dependents: DownstreamScope::Deep,
                    ..RunRequirements::default()
                };

                // Inserts `c` as a dependency only, with no dependents expanded
                let task_b = wg.get_task_from_project("b", "task").unwrap();

                builder.run_task(&task_b, &reqs).await.unwrap();

                // Then runs `c` as an explicit target, which must expand them
                let task_c = wg.get_task_from_project("c", "task").unwrap();

                builder.run_task(&task_c, &reqs).await.unwrap();

                let (_, graph) = builder.build();

                assert_eq!(
                    extract_run_task_targets(graph),
                    ["a:task", "b:task", "c:task", "d:task", "e:task"]
                );
            }
        }
    }

    mod run_task_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_deps_in_parallel() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "parallel").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:parallel").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_deps_in_serial() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "serial").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:serial").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_create_a_chain() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "chain1").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:chain1").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_include_dependents() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "base").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:base").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_dependents() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "base").unwrap();

            builder
                .run_task(
                    &task,
                    &RunRequirements {
                        dependents: DownstreamScope::Deep,
                        ..RunRequirements::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:base").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_dependents_for_ci() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "base").unwrap();

            builder
                .run_task(
                    &task,
                    &RunRequirements {
                        ci: true,
                        ci_check: true,
                        dependents: DownstreamScope::Deep,
                        ..RunRequirements::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("deps:base").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn serial_deps_dont_cycle_on_shared_nodes() {
            let sandbox = create_sandbox("serial-cycle");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // Run all :test tasks — this previously caused a WouldCycle error
            // because serial dep chains added ordering edges to pre-existing
            // nodes created by other tasks' dependency resolution.
            let tasks: Vec<_> = ["lib-a", "lib-b", "lib-c", "app-d"]
                .iter()
                .map(|id| wg.get_task_from_project(id, "test").unwrap())
                .collect();

            for task in &tasks {
                builder
                    .run_task(task, &RunRequirements::default())
                    .await
                    .unwrap();
            }

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn serial_deps_order_grandchildren() {
            let sandbox = create_sandbox("serial-subtree");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // build => [clean, build-tasks] (serial). build-tasks has its own
            // deps (cli:build-library-bundle-cli, tsc-project, prepare-package),
            // all of which must run after clean — not just build-tasks itself.
            let task = wg.get_task_from_project("app", "build").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("app:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn serial_subtree_doesnt_escape_via_shared_node() {
            let sandbox = create_sandbox("serial-shared");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // parent1 => [p, b] and parent2 => [q, b], both serial, sharing the
            // node `b`. parent1 adds a serial `b -> p` edge; walking `b` for
            // parent2 must not follow it into p/pchild and order them after `q`.
            // The snapshot must contain no `p -> q` or `pchild -> q` edges.
            for name in ["parent1", "parent2"] {
                let task = wg.get_task_from_project("proj", name).unwrap();

                builder
                    .run_task(&task, &RunRequirements::default())
                    .await
                    .unwrap();
            }

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
        }
    }

    mod dep_types {
        use super::*;

        fn find_task_index(graph: &ActionGraph, target: &str) -> NodeIndex {
            graph
                .get_inner_nodes()
                .iter()
                .find_map(|(index, node)| match node {
                    ActionNode::RunTask(inner) if inner.target.as_str() == target => Some(*index),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("No node for target {target}!"))
        }

        fn map_targets(targets: Vec<Target>) -> Vec<String> {
            let mut targets = targets
                .into_iter()
                .map(|target| target.to_string())
                .collect::<Vec<_>>();
            targets.sort();
            targets
        }

        fn map_edges(graph: &ActionGraph) -> Vec<(String, String, String)> {
            let inner = graph.get_inner_graph();

            inner
                .graph()
                .edge_indices()
                .map(|edge| {
                    let (source, target) = inner.edge_endpoints(edge).unwrap();

                    (
                        graph.get_node_from_index(&source).unwrap().label(),
                        graph.get_node_from_index(&target).unwrap().label(),
                        inner.edge_weight(edge).unwrap().to_string(),
                    )
                })
                .collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_required_and_cleanup_deps() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "base").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());

            // The cleanup edge is reversed, it runs after the task
            let edges = map_edges(&graph);

            assert!(edges.contains(&(
                "RunTask(proj:base)".into(),
                "RunTask(proj:setup)".into(),
                "required".into()
            )));
            assert!(edges.contains(&(
                "RunTask(proj:teardown)".into(),
                "RunTask(proj:base)".into(),
                "cleanup".into()
            )));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn tracks_cleanup_indices_through_build() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "base").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            let teardown = find_task_index(&graph, "proj:teardown");

            assert_eq!(
                graph.get_cleanup_indices().iter().collect::<Vec<_>>(),
                vec![&teardown]
            );
            assert!(graph.is_cleanup_index(&teardown));
            assert!(!graph.is_cleanup_index(&find_task_index(&graph, "proj:base")));
            assert!(!graph.is_cleanup_index(&find_task_index(&graph, "proj:setup")));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_wait_deps() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "waits").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());

            assert!(map_edges(&graph).contains(&(
                "RunTask(proj:waits)".into(),
                "RunPersistentTask(proj:server)".into(),
                "wait".into()
            )));
            assert!(graph.get_cleanup_indices().is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_serial_deps_of_all_types() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "serial").unwrap();

            builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());

            let edges = map_edges(&graph);
            let task_edges = edges
                .iter()
                .filter(|(source, target, _)| {
                    source.contains("Task(proj:") && target.contains("Task(proj:")
                })
                .cloned()
                .collect::<Vec<_>>();

            // Only the required deps are chained (b runs after a), while the
            // wait and cleanup deps take no part in the chain
            assert_eq!(
                task_edges,
                vec![
                    (
                        "RunTask(proj:b)".into(),
                        "RunTask(proj:a)".into(),
                        "required".into()
                    ),
                    (
                        "RunTask(proj:serial)".into(),
                        "RunTask(proj:a)".into(),
                        "required".into()
                    ),
                    (
                        "RunTask(proj:serial)".into(),
                        "RunPersistentTask(proj:server)".into(),
                        "wait".into()
                    ),
                    (
                        "RunTask(proj:serial)".into(),
                        "RunTask(proj:b)".into(),
                        "required".into()
                    ),
                    (
                        "RunTask(proj:teardown)".into(),
                        "RunTask(proj:serial)".into(),
                        "cleanup".into()
                    ),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn shares_a_cleanup_between_tasks() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            for id in ["base", "other"] {
                let task = wg.get_task_from_project("proj", id).unwrap();

                builder
                    .run_task(&task, &RunRequirements::default())
                    .await
                    .unwrap();
            }

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());

            let edges = map_edges(&graph);

            assert!(edges.contains(&(
                "RunTask(proj:teardown)".into(),
                "RunTask(proj:base)".into(),
                "cleanup".into()
            )));
            assert!(edges.contains(&(
                "RunTask(proj:teardown)".into(),
                "RunTask(proj:other)".into(),
                "cleanup".into()
            )));
            assert_eq!(graph.get_cleanup_indices().len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn expands_cleanups_as_dependents() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "base").unwrap();

            builder
                .run_task(
                    &task,
                    &RunRequirements {
                        dependents: DownstreamScope::Direct,
                        ..RunRequirements::default()
                    },
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            // The cleanup task is a "dependent" of the task it cleans up, but
            // it was already inserted as a dependency, so it must not be
            // duplicated, nor gain a second edge
            let edges = map_edges(&graph);

            assert_eq!(
                edges
                    .iter()
                    .filter(|(source, target, _)| source == "RunTask(proj:teardown)"
                        && target.starts_with("RunTask"))
                    .collect::<Vec<_>>(),
                vec![&(
                    "RunTask(proj:teardown)".into(),
                    "RunTask(proj:base)".into(),
                    "cleanup".into()
                )]
            );
            assert_eq!(
                graph
                    .get_inner_nodes()
                    .values()
                    .filter(|node| node.label() == "RunTask(proj:teardown)")
                    .count(),
                1
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_when_a_cleanup_would_cycle() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // parent -> mid -> cleanup, and then cleanup -> parent
            let mut task = wg
                .get_task_from_project("proj", "cycle-parent")
                .unwrap()
                .as_ref()
                .to_owned();

            task.deps.push(TaskDependencyConfig {
                target: Target::parse("proj:cycle-cleanup").unwrap(),
                type_of: TaskDependencyType::Cleanup,
                ..TaskDependencyConfig::default()
            });

            let error = builder
                .run_task(&task, &RunRequirements::default())
                .await
                .unwrap_err();

            assert!(
                error.to_string().contains(
                    "adding a relationship from action RunTask(proj:cycle-cleanup) to RunTask(proj:cycle-parent) would introduce a cycle"
                ),
                "{error}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn task_graph_relations_are_reversed_for_cleanups() {
            let sandbox = create_sandbox("dep-types");
            let container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;

            let base = wg.get_task_from_project("proj", "base").unwrap();
            let teardown = wg.get_task_from_project("proj", "teardown").unwrap();

            // The cleanup task is a dependent of the task it cleans up
            let mut dependents = map_targets(wg.tasks.dependents_of(base.as_ref()));

            assert_eq!(dependents, vec!["proj:teardown"]);

            assert_eq!(
                map_targets(wg.tasks.dependencies_of(base.as_ref())),
                vec!["proj:setup"]
            );

            // And the task is a dependency of its cleanup
            dependents = map_targets(wg.tasks.dependents_of(teardown.as_ref()));

            assert!(dependents.is_empty());

            assert_eq!(
                map_targets(wg.tasks.dependencies_of(teardown.as_ref())),
                vec!["proj:base", "proj:other", "proj:serial"]
            );
        }
    }

    mod run_task_by_target {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Dependencies scope (^:) is not supported in run contexts.")]
        async fn errors_on_parent_scope() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("^:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Self scope (~:) is not supported in run contexts.")]
        async fn errors_on_self_scope() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("~:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }
        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(
            expected = "No project has been configured with the identifier or alias unknown."
        )]
        async fn errors_for_unknown_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("unknown:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task unknown for project server.")]
        async fn errors_for_unknown_project_task() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("server:unknown").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task internal for project common.")]
        async fn errors_for_internal_task_when_explicit() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("common:internal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_all() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("client:build").unwrap(),
                    Target::parse("base:build").unwrap(),
                    Target::parse("common:build").unwrap(),
                    Target::parse("server:build").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_all_with_query() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_query("language=rust").unwrap();

            builder
                .run_task_by_target(
                    Target::parse(":build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("server:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_all_no_nodes() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":unknown").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_all_internal() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":internal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("client:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("client:lint").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_for_internal_task_when_depended_on() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("misc:requiresInternal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("misc:requiresInternal").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_tag() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("#frontend:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("client:lint").unwrap(),
                    Target::parse("common:lint").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_tag_no_nodes() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("#unknown:lint").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_tags_internal() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("#frontend:internal").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_task_tag_for_project() {
            // `project:#tasktag` — explicit project, task tag
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("client:#quality").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert!(!graph.is_empty());
            // Both `client:lint` and `client:test` are tagged "quality"
            let dot = graph.to_dot();
            assert!(dot.contains("client:lint"));
            assert!(dot.contains("client:test"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_task_tag_all_scope() {
            // `:#tasktag` — all projects, task tag
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":#quality").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();
            let dot = graph.to_dot();

            // client:lint, client:test, common:lint should be selected
            // common:internal is tagged but is internal, so excluded
            assert!(dot.contains("client:lint"));
            assert!(dot.contains("client:test"));
            assert!(dot.contains("common:lint"));
            assert!(!dot.contains("common:internal"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_task_tag_with_project_tag_scope() {
            // `#projtag:#tasktag` — project tag + task tag
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("#frontend:#quality").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (_, graph) = builder.build();
            let dot = graph.to_dot();

            // #frontend matches client and common; quality matches lint/test
            assert!(dot.contains("client:lint"));
            assert!(dot.contains("client:test"));
            assert!(dot.contains("common:lint"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn task_tag_with_no_match_in_explicit_project_is_empty() {
            // `project:#tasktag` where project exists but no tagged tasks → empty result
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse("server:#quality").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn task_tag_with_unknown_tag_is_empty() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target(
                    Target::parse(":#unknown-tag").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert!(graph.is_empty());
            assert!(context.primary_targets.is_empty());
        }
    }

    mod run_task_by_target_locator {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task internal for project common.")]
        async fn errors_for_internal_task_when_explicit() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::Qualified(Target::parse("common:internal").unwrap()),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_target() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::Qualified(Target::parse("server:build").unwrap()),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("server:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_task_glob() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse(":*-dependency").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse(":{a,c}").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("deps-affected:c").unwrap(),
                    Target::parse("deps:a").unwrap(),
                    Target::parse("deps:c").unwrap(),
                    Target::parse("ci:ci2-dependency").unwrap(),
                    Target::parse("ci:ci3-dependency").unwrap(),
                    Target::parse("ci:ci4-dependency").unwrap(),
                    Target::parse("deps-affected:a").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_tag_glob() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse("#front*:build").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("client:build").unwrap(),
                    Target::parse("common:build").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_task_tag_glob() {
            // `:#tasktag-*` — all projects, task tag glob (routed through MQL `taskTag~`)
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse(":#qual*").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();
            let dot = graph.to_dot();

            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("client:lint").unwrap(),
                    Target::parse("client:test").unwrap(),
                    Target::parse("common:lint").unwrap(),
                ]
            );

            // common:internal is tagged but is internal, so excluded
            assert!(!dot.contains("common:internal"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_project_tag_and_task_tag_glob() {
            // `#projtag-*:#tasktag-*` — project tag glob + task tag glob
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse("#front*:#qual*").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, _) = builder.build();

            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("client:lint").unwrap(),
                    Target::parse("client:test").unwrap(),
                    Target::parse("common:lint").unwrap(),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_empty_result_for_task_tag_glob_no_match() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse(":#unknown-*").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, _) = builder.build();

            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_project_glob() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse("c{lient,ommon}:test").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("client:test").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_empty_result_for_no_glob_match() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::parse("{foo,bar}:task-*").unwrap(),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_for_internal_task_when_depended_on() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::Qualified(Target::parse("misc:requiresInternal").unwrap()),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("misc:requiresInternal").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_in_default_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_workspace_config(|config| {
                config.default_project = Some(Id::raw("base"));
            });

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::DefaultProject(Id::raw("build")),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("base:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "No default project has been configured")]
        async fn errors_for_no_default_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::DefaultProject(Id::raw("build")),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Invalid default project")]
        async fn errors_for_invalid_default_project() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_workspace_config(|config| {
                config.default_project = Some(Id::raw("unknown"));
            });

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_task_by_target_locator(
                    TargetLocator::DefaultProject(Id::raw("build")),
                    &RunRequirements::default(),
                )
                .await
                .unwrap();
        }
    }

    mod run_tasks {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_partition_if_no_job() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            // 0
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements::default(),
                )
                .await
                .unwrap();

            let (context, _) = builder.build();

            assert_eq!(context.primary_targets.len(), 10);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn partitions_by_job() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            // 0
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements {
                        job: Some(0),
                        job_total: Some(3),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_eq!(context.primary_targets.len(), 4);
            assert_snapshot!(graph.to_dot());

            // 1
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements {
                        job: Some(1),
                        job_total: Some(3),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_eq!(context.primary_targets.len(), 4);
            assert_snapshot!(graph.to_dot());

            // 2
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder
                .run_tasks(
                    vec![TargetLocator::parse("partition:task-*").unwrap()],
                    RunRequirements {
                        job: Some(2),
                        job_total: Some(3),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();

            let (context, graph) = builder.build();

            assert_eq!(context.primary_targets.len(), 2);
            assert_snapshot!(graph.to_dot());
        }
    }

    mod setup_env {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_if_supported() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            let index = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            assert!(index.is_some());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple_projects() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let bar = wg.get_project("bar").unwrap();
            builder
                .setup_environment(&spec, &bar.source, &bar)
                .await
                .unwrap();

            let baz = wg.get_project("baz").unwrap();
            builder
                .setup_environment(&spec, &baz.source, &baz)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("baz")),
                        root: WorkspaceRelativePathBuf::from("baz"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            let index1 = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();
            let index2 = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            assert_eq!(index1, index2);

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_unsupported() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // Supports tier 2 but not `setup_environment`
            let spec = create_tier_spec(2);

            let project = wg.get_project("bar").unwrap();
            let index = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_environment: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_environment: PipelineActionSwitch::Only(vec![Id::raw("rust")]),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_environment: PipelineActionSwitch::Only(vec![Id::raw(
                            "tc-tier2-setup-env",
                        )]),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-setup-env")) {
                    inner.config.insert(
                        "testEnvRequirements".into(),
                        serde_json::json!(["tc-tier3"]),
                    );
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            let index = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            assert!(index.is_some());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains_when_no_setup_environment_itself() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-reqs")) {
                    inner
                        .config
                        .insert("testRequiresForEnvironment".into(), serde_json::json!(true));
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // Doesn't implement `setup_environment`, so the action is
            // only created to anchor the required toolchains
            let spec = create_tier_spec_with_name("tc-tier2-reqs");

            let project = wg.get_project("bar").unwrap();
            let index = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            assert!(index.is_some());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: spec.clone(),
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_requirements_not_for_environment() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // Has requirements, but only for `setup_toolchain`,
            // and doesn't implement `setup_environment`
            let spec = create_tier_spec_with_name("tc-tier2-reqs");

            let project = wg.get_project("bar").unwrap();
            let index = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_if_required_toolchain_not_configured() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-setup-env")) {
                    inner.config.insert(
                        "testEnvRequirements".into(),
                        serde_json::json!(["tc-unknown"]),
                    );
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();

            let error = builder
                .setup_environment(&spec, &project.source, &project)
                .await
                .unwrap_err();

            assert_eq!(
                error.to_string(),
                "Toolchain tc-tier2-setup-env requires the toolchain tc-unknown, but it has not been configured!"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains_from_project_config() {
            let sandbox = create_sandbox("projects");

            // Only the bar project defines environment requirements
            sandbox.create_file(
                "bar/moon.yml",
                "language: javascript\ntoolchains:\n  tc-tier2-setup-env:\n    testEnvRequirements: ['tc-tier3']\n",
            );

            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let bar = wg.get_project("bar").unwrap();
            builder
                .setup_environment(&spec, &bar.source, &bar)
                .await
                .unwrap();

            let baz = wg.get_project("baz").unwrap();
            builder
                .setup_environment(&spec, &baz.source, &baz)
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: Some(Id::raw("baz")),
                        root: WorkspaceRelativePathBuf::from("baz"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }
    }

    mod setup_env_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_if_deps_root_is_workspace_root() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let index = builder.setup_environment_root(&spec).await.unwrap();

            assert!(index.is_some());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-setup-env")) {
                    inner.config.insert(
                        "testEnvRequirements".into(),
                        serde_json::json!(["tc-tier3"]),
                    );
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let index = builder.setup_environment_root(&spec).await.unwrap();

            assert!(index.is_some());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_deps_root_isnt_workspace_root() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd
                .set_working_dir(sandbox.path().join("bar"));

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let index = builder.setup_environment_root(&spec).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_unsupported() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // Supports tier 2 but not `setup_environment`
            let spec = create_tier_spec(2);

            let index = builder.setup_environment_root(&spec).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec(1);

            let index = builder.setup_environment_root(&spec).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_environment: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            builder.setup_environment_root(&spec).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }

    mod setup_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier1"));

            builder.setup_toolchain(&ts, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier2() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier2"));

            builder.setup_toolchain(&ts, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_same_toolchain() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node1 = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );
            let node2 = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(4, 5, 6)),
            );
            let node3 = ToolchainSpec::new_global(Id::raw("tc-tier3"));
            let node4 = node1.clone();
            let node5 = node2.clone();

            builder.setup_toolchain(&node1, None).await.unwrap();
            builder.setup_toolchain(&node2, None).await.unwrap();
            builder.setup_toolchain(&node3, None).await.unwrap();
            builder.setup_toolchain(&node4, None).await.unwrap();
            builder.setup_toolchain(&node5, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node1 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node2 }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node3 }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = create_tier_spec(3);

            builder.setup_toolchain(&node, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: PipelineActionSwitch::Only(vec![Id::raw("system")]),
                        ..Default::default()
                    },
                )
                .await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        setup_toolchains: PipelineActionSwitch::Only(vec![
                            Id::raw("system"),
                            Id::raw("tc-tier3"),
                        ]),
                        ..Default::default()
                    },
                )
                .await;

            let system = ToolchainSpec::system();
            let node = ToolchainSpec::new(
                Id::raw("tc-tier3"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&system, None).await.unwrap();
            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = ToolchainSpec::new(
                Id::raw("tc-tier3-reqs"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: node }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_require_other_toolchains_when_no_setup_toolchain_itself() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let node = ToolchainSpec::new(
                Id::raw("tc-tier2-reqs"),
                create_unresolved_version(Version::new(1, 2, 3)),
            );

            builder.setup_toolchain(&node, None).await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier3"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: ToolchainSpec::new(
                            Id::raw("tc-tier2-reqs"),
                            create_unresolved_version(Version::new(1, 2, 3)),
                        )
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_require_other_toolchains_if_not_for_setup_toolchain() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-setup-env")) {
                    inner.config.insert(
                        "testEnvRequirements".into(),
                        serde_json::json!(["tc-tier3"]),
                    );
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // Has requirements, but only for `setup_environment`,
            // and doesn't support tier 3 itself
            let node = create_tier_spec_with_name("tc-tier2-setup-env");

            let index = builder.setup_toolchain(&node, None).await.unwrap();

            assert!(index.is_none());

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }

    mod sync_project {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_single() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let bar = wg.get_project("bar").unwrap();
            builder
                .sync_project(&bar, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let foo = wg.get_project("foo").unwrap();
            builder
                .sync_project(&foo, &RunRequirements::default())
                .await
                .unwrap();

            let bar = wg.get_project("bar").unwrap();
            builder
                .sync_project(&bar, &RunRequirements::default())
                .await
                .unwrap();

            let qux = wg.get_project("qux").unwrap();
            builder
                .sync_project(&qux, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("qux"),
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_without_deps() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_project_dependencies: false,
                        ..Default::default()
                    },
                )
                .await;

            let foo = wg.get_project("foo").unwrap();
            builder
                .sync_project(&foo, &RunRequirements::default())
                .await
                .unwrap();

            let qux = wg.get_project("qux").unwrap();
            builder
                .sync_project(&qux, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("qux"),
                    }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let foo = wg.get_project("foo").unwrap();

            builder
                .sync_project(&foo, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .sync_project(&foo, &RunRequirements::default())
                .await
                .unwrap();
            builder
                .sync_project(&foo, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    }),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("foo"),
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_projects: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let bar = wg.get_project("bar").unwrap();
            builder
                .sync_project(&bar, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_not_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_projects: PipelineActionSwitch::Only(vec![Id::raw("foo")]),
                        ..Default::default()
                    },
                )
                .await;

            let bar = wg.get_project("bar").unwrap();
            builder
                .sync_project(&bar, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_listed() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container
                .create_builder_with_options(
                    wg.clone(),
                    ActionGraphBuilderOptions {
                        sync_projects: PipelineActionSwitch::Only(vec![Id::raw("bar")]),
                        ..Default::default()
                    },
                )
                .await;

            let bar = wg.get_project("bar").unwrap();
            builder
                .sync_project(&bar, &RunRequirements::default())
                .await
                .unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("bar"),
                    })
                ]
            );
        }
    }

    mod sync_workspace {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_dupes() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.sync_workspace().await.unwrap();
            builder.sync_workspace().await.unwrap();
            builder.sync_workspace().await.unwrap();

            let (_, graph) = builder.build();

            assert_eq!(topo(graph), vec![ActionNode::sync_workspace()]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder_with_options(
                    container.create_workspace_graph().await,
                    ActionGraphBuilderOptions {
                        sync_workspace: false,
                        ..Default::default()
                    },
                )
                .await;

            builder.sync_workspace().await.unwrap();

            let (_, graph) = builder.build();

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }

    mod run_tasks_with_plan {
        use super::*;

        mod included_targets {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_included_targets() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Included(vec![
                        TargetLocator::parse("base:build").unwrap(),
                    ]),
                    ..Default::default()
                };

                let partition = builder
                    .run_tasks_with_plan(&plan, RunRequirements::default())
                    .await
                    .unwrap();

                assert_eq!(partition.targets.len(), 1);

                let (context, graph) = builder.build();

                assert_eq!(context.primary_targets.len(), 1);
                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_multiple_included_targets() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Included(vec![
                        TargetLocator::parse("base:build").unwrap(),
                        TargetLocator::parse("common:lint").unwrap(),
                    ]),
                    ..Default::default()
                };

                let partition = builder
                    .run_tasks_with_plan(&plan, RunRequirements::default())
                    .await
                    .unwrap();

                assert_eq!(partition.targets.len(), 2);

                let (context, graph) = builder.build();

                assert_eq!(context.primary_targets.len(), 2);
                assert_snapshot!(graph.to_dot());
            }
        }

        mod filtered_targets {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_filtered_include() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Filtered {
                        include: vec![TargetLocator::parse("base:build").unwrap()],
                    },
                    ..Default::default()
                };

                let partition = builder
                    .run_tasks_with_plan(&plan, RunRequirements::default())
                    .await
                    .unwrap();

                assert_eq!(partition.targets.len(), 1);

                let (context, graph) = builder.build();

                assert_eq!(context.primary_targets.len(), 1);
                assert_snapshot!(graph.to_dot());
            }
        }

        mod partitioned_targets {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_specific_job_partition() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Partitioned {
                        jobs: vec![
                            vec![TargetLocator::parse("base:build").unwrap()],
                            vec![TargetLocator::parse("common:lint").unwrap()],
                        ],
                    },
                    ..Default::default()
                };

                let partition = builder
                    .run_tasks_with_plan(
                        &plan,
                        RunRequirements {
                            job: Some(0),
                            job_total: Some(2),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                assert_eq!(partition.targets.len(), 1);
                assert_eq!(partition.size, Some(1));

                let (context, graph) = builder.build();

                assert_eq!(context.primary_targets.len(), 1);
                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_second_job_partition() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Partitioned {
                        jobs: vec![
                            vec![TargetLocator::parse("base:build").unwrap()],
                            vec![TargetLocator::parse("common:lint").unwrap()],
                        ],
                    },
                    ..Default::default()
                };

                let partition = builder
                    .run_tasks_with_plan(
                        &plan,
                        RunRequirements {
                            job: Some(1),
                            job_total: Some(2),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                assert_eq!(partition.targets.len(), 1);
                assert_eq!(partition.size, Some(1));

                let (context, graph) = builder.build();

                assert_eq!(context.primary_targets.len(), 1);
                assert_snapshot!(graph.to_dot());
            }

            #[should_panic(expected = "pipeline has not been configured for parallelism")]
            #[tokio::test(flavor = "multi_thread")]
            async fn errors_without_job_and_job_total() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Partitioned {
                        jobs: vec![
                            vec![TargetLocator::parse("base:build").unwrap()],
                            vec![TargetLocator::parse("common:lint").unwrap()],
                        ],
                    },
                    ..Default::default()
                };

                builder
                    .run_tasks_with_plan(&plan, RunRequirements::default())
                    .await
                    .unwrap();
            }

            #[should_panic(expected = "invalid job index was provided")]
            #[tokio::test(flavor = "multi_thread")]
            async fn errors_when_job_index_exceeds_total() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Partitioned {
                        jobs: vec![
                            vec![TargetLocator::parse("base:build").unwrap()],
                            vec![TargetLocator::parse("common:lint").unwrap()],
                        ],
                    },
                    ..Default::default()
                };

                builder
                    .run_tasks_with_plan(
                        &plan,
                        RunRequirements {
                            job: Some(5),
                            job_total: Some(2),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();
            }

            #[should_panic(expected = "pipeline has been configured for 5 jobs")]
            #[tokio::test(flavor = "multi_thread")]
            async fn errors_when_job_total_mismatches_partitions() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    targets: TargetsBlock::Partitioned {
                        jobs: vec![
                            vec![TargetLocator::parse("base:build").unwrap()],
                            vec![TargetLocator::parse("common:lint").unwrap()],
                        ],
                    },
                    ..Default::default()
                };

                builder
                    .run_tasks_with_plan(
                        &plan,
                        RunRequirements {
                            job: Some(0),
                            job_total: Some(5),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();
            }
        }

        mod graph_options {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn applies_upstream_from_plan() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    graph: GraphBlock {
                        upstream: Some(UpstreamScope::Deep),
                        ..Default::default()
                    },
                    targets: TargetsBlock::Included(vec![
                        TargetLocator::parse("common:build").unwrap(),
                    ]),
                    ..Default::default()
                };

                builder
                    .run_tasks_with_plan(
                        &plan,
                        RunRequirements {
                            dependencies: plan.graph.upstream.unwrap_or_default(),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn applies_downstream_from_plan() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    graph: GraphBlock {
                        downstream: Some(DownstreamScope::Direct),
                        ..Default::default()
                    },
                    targets: TargetsBlock::Included(vec![
                        TargetLocator::parse("base:build").unwrap(),
                    ]),
                    ..Default::default()
                };

                builder
                    .run_tasks_with_plan(
                        &plan,
                        RunRequirements {
                            dependents: plan.graph.downstream.unwrap_or_default(),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                let (_, graph) = builder.build();

                assert_snapshot!(graph.to_dot());
            }
        }
    }
}
