mod utils;

use moon_action::*;
use moon_action_context::{ActionContext, TargetState};
use moon_action_graph::{
    ActionGraph, ActionGraphBuilderOptions, BuildOutput, RunPartition, RunRequirements,
};
use moon_affected::AffectedBy;
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{
    EnvMap, PROTO_CLI_VERSION, PipelineActionSwitch, TaskDependencyConfig, TaskDependencyType,
    TaskOptionRunInCI, UnresolvedVersionSpec, Version, VersionSpec,
};
use moon_exec_plan::{ExecutionPlan, GraphBlock, TargetsBlock};
use moon_graph_utils::*;
use moon_task::{Target, TargetLocator, Task, TaskFileInput};
use moon_toolchain::ToolchainSpec;
use moon_workspace_graph::WorkspaceGraph;
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{assert_snapshot, create_sandbox};
use std::sync::Arc;
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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

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
                        owner_project_id: None,
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
            builder.install_dependencies(&spec1, project.clone());
            let index2 = builder.install_dependencies(&spec2, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            // The install runs with the root's version, while the other
            // version is still setup, for the request that asked for it
            assert_eq!(output.report.nodes(index2).len(), 2);
            assert_eq!(
                graph.dependencies_of(&ActionNode::install_dependencies(InstallDependenciesNode {
                    members: None,
                    owner_project_id: None,
                    project_id: Some(Id::raw("bar")),
                    root: WorkspaceRelativePathBuf::from("bar"),
                    toolchain_id: spec1.id.clone(),
                })),
                vec!["SetupToolchain(tc-tier3:1.2.3)"]
            );

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: spec1.clone()
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec1.id,
                    }),
                    ActionNode::setup_toolchain(SetupToolchainNode { toolchain: spec2 }),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple_toolchain_versions_with_setup_env() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                cfg.plugins
                    .entry(Id::raw("tc-tier3-setup-env"))
                    .or_default();
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec1 = create_tier_spec_with_name("tc-tier3-setup-env");

            let mut spec2 = create_tier_spec_with_name("tc-tier3-setup-env");
            spec2.req = Some(create_unresolved_version(Version::new(4, 5, 6)));

            // Both versions resolve to the same environment, but only the
            // root's version (the project's own) is involved in its actions
            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec1, project.clone());
            builder.install_dependencies(&spec2, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());

            // The environment is setup with the root's version only
            let env_node = ActionNode::setup_environment(SetupEnvironmentNode {
                owner_project_id: None,
                project_id: Some(Id::raw("bar")),
                root: WorkspaceRelativePathBuf::from("bar"),
                toolchain_id: spec1.id.clone(),
            });
            let mut env_deps = graph.dependencies_of(&env_node);
            env_deps.sort();

            assert_eq!(
                env_deps,
                vec!["SetupToolchain(tc-tier3-setup-env:1.2.3)", "SyncWorkspace",]
            );
            assert!(
                graph
                    .get_nodes()
                    .iter()
                    .any(|node| { node.label() == "SetupToolchain(tc-tier3-setup-env:4.5.6)" })
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_multiple_toolchain_versions_using_overrides() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(
                &builder
                    .get_project_spec(&Id::raw("rust"), &project)
                    .unwrap(),
                project.clone(),
            );

            let project = wg.get_project("qux").unwrap();
            builder.install_dependencies(
                &builder
                    .get_project_spec(&Id::raw("rust"), &project)
                    .unwrap(),
                project.clone(),
            );

            let graph = builder.build().await.unwrap().graph;

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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

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
                        owner_project_id: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_setup_toolchain_if_disabled() {
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

            // Dependencies aren't installed, but the toolchain must still be
            // setup, so that its binaries are available to the project's tasks
            let spec = create_tier_spec(3);

            let project = wg.get_project("bar").unwrap();
            let index = builder.install_dependencies(&spec, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

            assert_snapshot!(graph.to_dot());
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
        async fn graphs_setup_toolchain_if_disabled_in_config() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            let spec = create_tier_spec(3);

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&spec.id) {
                    inner.install_dependencies = false;
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let project = wg.get_project("bar").unwrap();
            let index = builder.install_dependencies(&spec, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

            assert_snapshot!(graph.to_dot());
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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
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
            let index = builder.install_dependencies(&spec, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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
            let index = builder.install_dependencies(&spec, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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
            let index = builder.install_dependencies(&spec, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        owner_project_id: Some(Id::raw("root")),
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
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        owner_project_id: Some(Id::raw("root")),
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

            let index = builder.install_dependencies_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
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

            builder.install_dependencies_root(&spec);

            let graph = builder.build().await.unwrap().graph;

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
                        owner_project_id: None,
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

            builder.install_dependencies_root(&spec);

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
                        project_id: None,
                        root: WorkspaceRelativePathBuf::new(),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
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

            builder.install_dependencies_root(&spec);

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        owner_project_id: Some(Id::raw("root")),
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
            let project_index = builder.install_dependencies(&spec, project.clone());
            let root_index = builder.install_dependencies_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert_eq!(
                output.report.nodes(project_index),
                output.report.nodes(root_index)
            );

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: Some(vec!["in".into()]),
                        owner_project_id: Some(Id::raw("root")),
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

            let index = builder.install_dependencies_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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

            let index = builder.install_dependencies_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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

            builder.install_dependencies_root(&spec);

            let graph = builder.build().await.unwrap().graph;

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

            builder.install_dependencies_root(&spec);

            let graph = builder.build().await.unwrap().graph;

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

            builder.install_dependencies_root(&spec);

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

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
        async fn sets_up_toolchain_if_installing_deps_is_disabled() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder_with_options(
                    container.create_workspace_graph().await,
                    ActionGraphBuilderOptions {
                        install_dependencies: false.into(),
                        ..Default::default()
                    },
                )
                .await;

            let mut task = create_task("bar", "build");
            task.toolchains = vec![Id::raw("tc-tier3")];

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

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
                        toolchain: create_tier_spec(3),
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements {
                interactive: true,
                ..Default::default()
            });
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            // Separate nodes
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["a".into(), "b".into(), "c".into()],
                    ..TaskDependencyConfig::default()
                },
            );
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["x".into(), "y".into(), "z".into()],
                    ..TaskDependencyConfig::default()
                },
            );

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["a".into(), "b".into(), "c".into()],
                    ..TaskDependencyConfig::default()
                },
            );
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["a".into(), "b".into(), "c".into()],
                    ..TaskDependencyConfig::default()
                },
            );

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["a".into(), "b".into(), "c".into()],
                    ..TaskDependencyConfig::default()
                },
            );
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["a".into(), "b".into(), "c".into()],
                    ..TaskDependencyConfig::default()
                },
            );

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            // Separate nodes
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                    ..TaskDependencyConfig::default()
                },
            );
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                    ..TaskDependencyConfig::default()
                },
            );

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                    ..TaskDependencyConfig::default()
                },
            );
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                    ..TaskDependencyConfig::default()
                },
            );

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());
            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            // Separate nodes
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["a".into(), "b".into(), "c".into()],
                    env: EnvMap::from_iter([("FOO".into(), Some("1".into()))]),
                    ..TaskDependencyConfig::default()
                },
            );
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["a".into(), "b".into(), "c".into()],
                    env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                    ..TaskDependencyConfig::default()
                },
            );
            builder.set_requirements(RunRequirements::default());
            builder.run_task_with_config(
                &task,
                &TaskDependencyConfig {
                    args: vec!["x".into(), "y".into(), "z".into()],
                    env: EnvMap::from_iter([("BAR".into(), Some("2".into()))]),
                    ..TaskDependencyConfig::default()
                },
            );

            let graph = builder.build().await.unwrap().graph;

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
                builder.set_changed_files(changed_files);
                builder.set_affected().unwrap();

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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

                builder.mock_affected(FxHashSet::from_iter([file]), Depth::NONE, |affected| {
                    affected
                        .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                        .unwrap();
                });

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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
                    Depth::NONE,
                    |affected| {
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..Default::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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

            // The `affected-starve` fixture covers marks that must not depend
            // on target insertion order. Previously, a task marked through
            // another task's relationship walk before its own visit never ran
            // its own checks and walks, starving transitive dependents of marks.
            #[tokio::test(flavor = "multi_thread")]
            async fn includes_deep_dependents_regardless_of_target_order() {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("base/src.txt")]),
                    Depth::Deep,
                    |_| {},
                );

                // base:test first, so its walk marks base:build before its visit
                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..Default::default()
                });
                builder.run_tasks(create_adverse_order_locators([
                    "base:test",
                    "base:build",
                    "mid:build",
                    "mid:test",
                    "top:build",
                    "top:test",
                ]));

                let graph = builder.build().await.unwrap().graph;

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
            async fn includes_deep_dependents_when_base_task_ordered_last() {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("base/src.txt")]),
                    Depth::Deep,
                    |_| {},
                );

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..Default::default()
                });
                builder.run_tasks(create_adverse_order_locators([
                    "mid:test",
                    "mid:build",
                    "top:test",
                    "top:build",
                    "base:test",
                    "base:build",
                ]));

                let graph = builder.build().await.unwrap().graph;

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
            async fn stays_monotonic_when_change_set_grows() {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.mock_affected(
                    FxHashSet::from_iter([
                        WorkspaceRelativePathBuf::from("base/src.txt"),
                        WorkspaceRelativePathBuf::from("top/src.txt"),
                    ]),
                    Depth::Deep,
                    |_| {},
                );

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..Default::default()
                });
                builder.run_tasks(create_adverse_order_locators([
                    "top:test",
                    "top:build",
                    "mid:test",
                    "mid:build",
                    "base:test",
                    "base:build",
                ]));

                let graph = builder.build().await.unwrap().graph;

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
            // dropped even though its dependency changed. When affected state
            // doesn't flow through relations (none depth), it's dropped as well
            async fn run_only_downstream_target(depth: Depth) -> Vec<String> {
                let sandbox = create_sandbox("affected-starve");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                // Only `base:build` is affected by this file
                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("base/src.txt")]),
                    depth,
                    |_| {},
                );

                // While `top:build` is the only requested target
                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..Default::default()
                });
                builder.run_tasks(vec![TargetLocator::Qualified(
                    Target::parse("top:build").unwrap(),
                )]);

                let graph = builder.build().await.unwrap().graph;

                extract_run_task_targets(graph)
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn marks_target_through_affected_depth() {
                assert_eq!(
                    run_only_downstream_target(Depth::Deep).await,
                    // `top:test` depends on `top:build`, so deep dependents
                    // pulls it in, while the dependencies reached through
                    // `top:build` do not expand their own dependents
                    ["base:build", "mid:build", "top:build", "top:test"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn marks_target_within_limited_affected_depth() {
                // `top:build` is 2 hops away from `base:build`, while
                // `top:test` is 3 hops away, so isn't affected
                assert_eq!(
                    run_only_downstream_target(Depth::Limit(2)).await,
                    ["base:build", "mid:build", "top:build"]
                );
                assert!(run_only_downstream_target(Depth::DIRECT).await.is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_mark_target_without_affected_depth() {
                assert!(run_only_downstream_target(Depth::NONE).await.is_empty());
            }

            // moonrepo/moon#2686: The affected depth only decides which of the
            // selected tasks are affected, and never adds tasks, so a task that is
            // affected through its dependency on the selected task isn't ran
            async fn run_publish(dependents: Depth) -> (ActionContext, Vec<String>) {
                let sandbox = create_sandbox("graph-depth");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.set_changed_files(FxHashSet::from_iter([WorkspaceRelativePathBuf::from(
                    "app/src.txt",
                )]));
                builder.track_affected(Depth::Deep, false);

                builder.set_requirements(RunRequirements {
                    dependents,
                    ..Default::default()
                });
                builder.run_tasks(vec![TargetLocator::parse("app:publish").unwrap()]);

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                (context, extract_run_task_targets(graph))
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_add_affected_dependents_without_dependents_depth() {
                let (context, targets) = run_publish(Depth::NONE).await;

                assert_eq!(targets, ["app:publish"]);

                // While it is affected through its dependency
                let affected = context.affected.unwrap();

                assert!(affected.is_task_affected(&Target::parse("app:deploy").unwrap()));
                assert!(
                    affected.tasks[&Target::parse("app:deploy").unwrap()]
                        .dependencies
                        .contains(&Target::parse("app:publish").unwrap())
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn adds_affected_dependents_with_dependents_depth() {
                let (_, targets) = run_publish(Depth::DIRECT).await;

                assert_eq!(targets, ["app:deploy", "app:publish"]);
            }

            async fn run_with_changed_files(
                locators: &[&str],
                changed_files: &[&str],
                skip_affected: bool,
            ) -> (Vec<String>, Vec<String>) {
                let sandbox = create_sandbox("graph-depth");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.set_changed_files(
                    changed_files
                        .iter()
                        .map(|file| WorkspaceRelativePathBuf::from(*file))
                        .collect(),
                );
                builder.track_affected(Depth::NONE, false);

                builder.set_requirements(RunRequirements {
                    skip_affected,
                    ..Default::default()
                });
                builder.run_tasks(
                    locators
                        .iter()
                        .map(|locator| TargetLocator::parse(locator).unwrap())
                        .collect::<Vec<_>>(),
                );

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                let mut affected = context
                    .affected
                    .unwrap()
                    .tasks
                    .into_keys()
                    .map(|target| target.to_string())
                    .collect::<Vec<_>>();
                affected.sort();

                (extract_run_task_targets(graph), affected)
            }

            // Skipping affected checks (`--force`) runs every selected task, but
            // they're still tracked, so that their affected state is reported
            #[tokio::test(flavor = "multi_thread")]
            async fn tracks_affected_tasks_when_skipping_affected_checks() {
                let (targets, affected) =
                    run_with_changed_files(&["app:publish", "a:deploy"], &["app/src.txt"], false)
                        .await;

                assert_eq!(targets, ["app:publish"]);
                assert_eq!(affected, ["app:publish"]);

                let (targets, affected) =
                    run_with_changed_files(&["app:publish", "a:deploy"], &["app/src.txt"], true)
                        .await;

                assert_eq!(targets, ["a:deploy", "app:publish"]);
                assert_eq!(affected, ["app:publish"]);
            }

            // Dependencies are inserted regardless of affected state, so they're
            // never checked (nor marked), whether affected checks are skipped or not
            #[tokio::test(flavor = "multi_thread")]
            async fn marks_the_same_tasks_when_skipping_affected_checks() {
                let changed_files = ["a/src.txt", "b/src.txt"];
                let (targets, affected) =
                    run_with_changed_files(&["b:deploy"], &changed_files, false).await;
                let (forced_targets, forced_affected) =
                    run_with_changed_files(&["b:deploy"], &changed_files, true).await;

                assert_eq!(targets, ["a:deploy", "b:deploy"]);
                assert_eq!(forced_targets, targets);
                assert_eq!(affected, ["b:deploy"]);
                assert_eq!(forced_affected, affected);
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

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    ..Default::default()
                });
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_dependents_if_both_are_ci_true() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci4-dependency").unwrap();

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    ..Default::default()
                });
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    ..Default::default()
                });
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: false,
                    ..Default::default()
                });
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

                builder.set_requirements(RunRequirements {
                    ci: false,
                    ci_check: false,
                    ..Default::default()
                });
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_dependents_if_both_are_ci_false() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("ci", "ci2-dependency").unwrap();

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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

        // Persistent tasks never complete, so in CI they're only ran when
        // another task depends on them, or when they've been requested.
        // Outside of CI, that's only the case when the targets weren't
        // requested, and were determined by the command (like `moon check`).
        // The `persistent-ci` fixture forms the following dependency graph,
        // where `server` and `proxy` are persistent and enabled for CI, and
        // `watcher` is persistent and not enabled for CI (the default):
        //   proxy -> server -> build
        //   watcher -> build
        //   e2e -> server (wait), stop (cleanup)
        mod persistent_in_ci {
            use super::*;

            fn ci_reqs() -> RunRequirements {
                RunRequirements {
                    ci: true,
                    ci_check: true,
                    ..RunRequirements::default()
                }
            }

            async fn run_targets(
                ids: &[&str],
                reqs: RunRequirements,
            ) -> (ActionContext, Vec<String>) {
                let sandbox = create_sandbox("persistent-ci");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                for id in ids {
                    let task = wg.get_task_from_project("proj", id).unwrap();

                    builder.set_requirements(reqs.clone());
                    builder.run_task(task.clone());
                }

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                (context, extract_run_task_targets(graph))
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_when_requested() {
                let (context, targets) = run_targets(&["proxy"], ci_reqs()).await;

                assert_eq!(targets, ["proj:build", "proj:proxy", "proj:server"]);
                assert!(
                    context
                        .primary_targets
                        .contains(&Target::parse("proj:proxy").unwrap())
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_as_a_dependency() {
                let (_, targets) = run_targets(&["e2e"], ci_reqs()).await;

                assert_eq!(
                    targets,
                    ["proj:build", "proj:e2e", "proj:server", "proj:stop"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_as_a_dependent() {
                let (_, targets) = run_targets(
                    &["build"],
                    RunRequirements {
                        dependents: Depth::DIRECT,
                        ..ci_reqs()
                    },
                )
                .await;

                assert_eq!(targets, ["proj:build"]);
            }

            // But the tasks that depend on it are, which then run it
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_the_dependents_of_a_dependent() {
                let (_, targets) = run_targets(
                    &["build"],
                    RunRequirements {
                        dependents: Depth::Deep,
                        ..ci_reqs()
                    },
                )
                .await;

                assert_eq!(
                    targets,
                    ["proj:build", "proj:e2e", "proj:server", "proj:stop"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_as_a_dependent_when_already_a_dependency() {
                let (_, targets) = run_targets(
                    &["e2e", "build"],
                    RunRequirements {
                        dependents: Depth::Deep,
                        ..ci_reqs()
                    },
                )
                .await;

                assert_eq!(
                    targets,
                    ["proj:build", "proj:e2e", "proj:server", "proj:stop"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_as_a_dependent_when_not_in_ci() {
                let (_, targets) = run_targets(
                    &["build"],
                    RunRequirements {
                        ci: false,
                        ci_check: true,
                        dependents: Depth::DIRECT,
                        ..RunRequirements::default()
                    },
                )
                .await;

                assert_eq!(targets, ["proj:build", "proj:server", "proj:watcher"]);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_as_a_dependent_when_ignoring_ci_checks() {
                let (_, targets) = run_targets(
                    &["build"],
                    RunRequirements {
                        ci: true,
                        ci_check: false,
                        dependents: Depth::DIRECT,
                        ..RunRequirements::default()
                    },
                )
                .await;

                assert_eq!(targets, ["proj:build", "proj:server", "proj:watcher"]);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_when_not_requested() {
                let (_, targets) = run_targets(
                    &["server", "proxy"],
                    RunRequirements {
                        skip_persistent: true,
                        ..ci_reqs()
                    },
                )
                .await;

                assert!(targets.is_empty());
            }

            // Like `moon ci` without targets, which runs every task
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_as_a_dependency_when_not_requested() {
                for ids in [
                    ["build", "server", "proxy", "watcher", "e2e", "stop"],
                    ["stop", "e2e", "watcher", "proxy", "server", "build"],
                ] {
                    let (context, targets) = run_targets(
                        &ids,
                        RunRequirements {
                            skip_persistent: true,
                            ..ci_reqs()
                        },
                    )
                    .await;

                    assert_eq!(
                        targets,
                        ["proj:build", "proj:e2e", "proj:server", "proj:stop"],
                        "{ids:?}"
                    );

                    // It was ran for another task, and not on its own
                    assert!(
                        !context
                            .primary_targets
                            .contains(&Target::parse("proj:server").unwrap()),
                        "{ids:?}"
                    );
                }
            }

            // Like `moon check`, which is also ran outside of CI
            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_when_not_requested_and_not_in_ci() {
                let (_, targets) = run_targets(
                    &["server", "proxy", "watcher"],
                    RunRequirements {
                        ci: false,
                        ci_check: true,
                        skip_persistent: true,
                        ..RunRequirements::default()
                    },
                )
                .await;

                assert!(targets.is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_as_a_dependent_when_not_requested_and_not_in_ci() {
                let (_, targets) = run_targets(
                    &["build"],
                    RunRequirements {
                        ci: false,
                        ci_check: true,
                        dependents: Depth::DIRECT,
                        skip_persistent: true,
                        ..RunRequirements::default()
                    },
                )
                .await;

                assert_eq!(targets, ["proj:build"]);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_as_a_dependency_when_not_requested_and_not_in_ci() {
                let (_, targets) = run_targets(
                    &["build", "server", "proxy", "watcher", "e2e", "stop"],
                    RunRequirements {
                        ci: false,
                        ci_check: true,
                        skip_persistent: true,
                        ..RunRequirements::default()
                    },
                )
                .await;

                assert_eq!(
                    targets,
                    ["proj:build", "proj:e2e", "proj:server", "proj:stop"]
                );
            }
        }

        mod dependencies {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn includes_deep_dependencies() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                assert!(context.ignored_dependencies.is_empty());
                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn includes_deep_dependencies_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    Depth::NONE,
                    |affected| {
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_skip_dependencies() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder.set_requirements(RunRequirements {
                    no_dependencies: true,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                assert_snapshot!(graph.to_dot());
                assert_eq!(
                    context.ignored_dependencies,
                    FxHashMap::from_iter([(
                        Target::parse("deps:chain3").unwrap(),
                        FxHashSet::from_iter([Target::parse("deps:chain4").unwrap()])
                    )])
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_skip_dependencies_affected() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps-affected", "b").unwrap();

                builder.mock_affected(
                    FxHashSet::from_iter([WorkspaceRelativePathBuf::from("deps-affected/b.txt")]),
                    Depth::NONE,
                    |affected| {
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder.set_requirements(RunRequirements {
                    no_dependencies: true,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }

            // Dependencies that were selected themselves are still linked,
            // regardless of the order that they're inserted in
            #[tokio::test(flavor = "multi_thread")]
            async fn links_selected_dependencies_when_skipping_dependencies() {
                for ids in [
                    ["chain1", "chain3", "chain2"],
                    ["chain3", "chain2", "chain1"],
                    ["chain2", "chain1", "chain3"],
                ] {
                    let sandbox = create_sandbox("tasks");
                    let mut container = ActionGraphContainer::new(sandbox.path());

                    let wg = container.create_workspace_graph().await;
                    let mut builder = container.create_builder(wg.clone()).await;

                    builder.set_requirements(RunRequirements {
                        no_dependencies: true,
                        ..RunRequirements::default()
                    });
                    builder.run_tasks(
                        ids.iter()
                            .map(|id| TargetLocator::parse(&format!("deps:{id}")).unwrap())
                            .collect::<Vec<_>>(),
                    );

                    let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                    assert_eq!(
                        topo(graph)
                            .into_iter()
                            .filter_map(|node| match node {
                                ActionNode::RunTask(inner) => Some(inner.target.to_string()),
                                _ => None,
                            })
                            .collect::<Vec<_>>(),
                        ["deps:chain3", "deps:chain2", "deps:chain1"],
                        "{ids:?}"
                    );
                    assert_eq!(
                        context.ignored_dependencies,
                        FxHashMap::from_iter([(
                            Target::parse("deps:chain3").unwrap(),
                            FxHashSet::from_iter([Target::parse("deps:chain4").unwrap()])
                        )]),
                        "{ids:?}"
                    );
                }
            }

            // moonrepo/moon#2685: `moon ci :deploy --no-dependencies`, where `c` depends
            // on `b`, which depends on `a`, and only `b` has changed
            #[tokio::test(flavor = "multi_thread")]
            async fn links_affected_dependencies_when_skipping_dependencies() {
                for locators in [vec![":deploy"], vec!["c:deploy", "b:deploy", "a:deploy"]] {
                    let sandbox = create_sandbox("graph-depth");
                    let mut container = ActionGraphContainer::new(sandbox.path());

                    let wg = container.create_workspace_graph().await;
                    let mut builder = container.create_builder(wg.clone()).await;

                    builder.set_changed_files(FxHashSet::from_iter([
                        WorkspaceRelativePathBuf::from("b/src.txt"),
                    ]));
                    builder.track_affected(Depth::Deep, true);

                    builder.set_requirements(RunRequirements {
                        ci: true,
                        ci_check: true,
                        no_dependencies: true,
                        ..RunRequirements::default()
                    });
                    builder.run_tasks(
                        locators
                            .iter()
                            .map(|locator| TargetLocator::parse(locator).unwrap())
                            .collect::<Vec<_>>(),
                    );

                    let BuildOutput { context, graph, .. } = builder.build().await.unwrap();
                    let inner = graph.get_inner_graph();
                    let edges = inner
                        .edge_indices()
                        .filter_map(|edge| {
                            let (source, target) = inner.edge_endpoints(edge).unwrap();

                            match (
                                graph.get_node_from_index(&source).unwrap(),
                                graph.get_node_from_index(&target).unwrap(),
                            ) {
                                (ActionNode::RunTask(source), ActionNode::RunTask(target)) => {
                                    Some((
                                        source.target.to_string(),
                                        target.target.to_string(),
                                        inner.edge_weight(edge).unwrap().to_string(),
                                    ))
                                }
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>();

                    assert_eq!(
                        edges,
                        [(
                            "c:deploy".to_owned(),
                            "b:deploy".to_owned(),
                            "required".to_owned()
                        )],
                        "{locators:?}"
                    );
                    assert_eq!(
                        extract_run_task_targets(graph),
                        ["b:deploy", "c:deploy"],
                        "{locators:?}"
                    );
                    assert_eq!(
                        context.ignored_dependencies,
                        FxHashMap::from_iter([(
                            Target::parse("b:deploy").unwrap(),
                            FxHashSet::from_iter([Target::parse("a:deploy").unwrap()])
                        )]),
                        "{locators:?}"
                    );
                }
            }

            // Multiple actions may run the same task (with different args or env),
            // and the dependencies of all of them must be linked
            mod multiple_actions {
                use super::*;

                fn describe(node: &ActionNode) -> String {
                    match node {
                        ActionNode::RunTask(inner) if !inner.args.is_empty() => {
                            format!("{} {}", inner.target, inner.args.join(" "))
                        }
                        ActionNode::RunTask(inner) => inner.target.to_string(),
                        other => other.label(),
                    }
                }

                fn map_task_edges(graph: &ActionGraph) -> Vec<(String, String, String)> {
                    let inner = graph.get_inner_graph();

                    let mut edges = inner
                        .edge_indices()
                        .filter_map(|edge| {
                            let (source, target) = inner.edge_endpoints(edge).unwrap();
                            let source = graph.get_node_from_index(&source).unwrap();
                            let target = graph.get_node_from_index(&target).unwrap();

                            (matches!(source, ActionNode::RunTask(_))
                                && matches!(target, ActionNode::RunTask(_)))
                            .then(|| {
                                (
                                    describe(source),
                                    describe(target),
                                    inner.edge_weight(edge).unwrap().to_string(),
                                )
                            })
                        })
                        .collect::<Vec<_>>();

                    edges.sort();
                    edges
                }

                fn edge(from: &str, to: &str, type_of: &str) -> (String, String, String) {
                    (from.to_owned(), to.to_owned(), type_of.to_owned())
                }

                fn target(id: &str) -> Target {
                    Target::parse(id).unwrap()
                }

                async fn build_graph(
                    ids: &[&str],
                    no_dependencies: bool,
                ) -> (ActionContext, ActionGraph) {
                    let sandbox = create_sandbox("dep-scopes");
                    let mut container = ActionGraphContainer::new(sandbox.path());

                    let wg = container.create_workspace_graph().await;
                    let mut builder = container.create_builder(wg.clone()).await;

                    let reqs = RunRequirements {
                        no_dependencies,
                        ..RunRequirements::default()
                    };

                    builder.set_requirements(reqs);

                    for id in ids {
                        let task = wg.get_task_from_project("proj", id).unwrap();

                        builder.run_task(task.clone());
                    }

                    let output = builder.build().await.unwrap();

                    (output.context, output.graph)
                }

                // The task is ran with args by another task, and on its own
                #[tokio::test(flavor = "multi_thread")]
                async fn links_deps_for_all_actions() {
                    for ids in [["with-args", "shared"], ["shared", "with-args"]] {
                        let (context, graph) = build_graph(&ids, false).await;

                        assert_eq!(
                            map_task_edges(&graph),
                            [
                                edge("proj:shared", "proj:prep", "required"),
                                edge("proj:shared --force", "proj:prep", "required"),
                                edge("proj:with-args", "proj:shared --force", "required"),
                            ],
                            "{ids:?}"
                        );
                        assert!(context.ignored_dependencies.is_empty(), "{ids:?}");
                    }
                }

                #[tokio::test(flavor = "multi_thread")]
                async fn links_deps_for_cleanup_actions() {
                    for ids in [["with-cleanup", "shared"], ["shared", "with-cleanup"]] {
                        let (context, graph) = build_graph(&ids, false).await;

                        assert_eq!(
                            map_task_edges(&graph),
                            [
                                edge("proj:shared", "proj:prep", "required"),
                                edge("proj:shared --force", "proj:prep", "required"),
                                edge("proj:shared --force", "proj:with-cleanup", "cleanup"),
                            ],
                            "{ids:?}"
                        );
                        assert!(context.ignored_dependencies.is_empty(), "{ids:?}");
                    }
                }

                // The dependency isn't in the graph with the configured args,
                // but it is without them, which must still run first
                #[tokio::test(flavor = "multi_thread")]
                async fn links_selected_deps_with_other_args_when_skipping_deps() {
                    for ids in [["with-args", "shared"], ["shared", "with-args"]] {
                        let (context, graph) = build_graph(&ids, true).await;

                        assert_eq!(
                            map_task_edges(&graph),
                            [edge("proj:with-args", "proj:shared", "required")],
                            "{ids:?}"
                        );
                        assert_eq!(
                            context.ignored_dependencies,
                            FxHashMap::from_iter([(
                                target("proj:shared"),
                                FxHashSet::from_iter([target("proj:prep")])
                            )]),
                            "{ids:?}"
                        );
                    }
                }

                // When the dependency is in the graph with the configured args
                // (as a cleanup of another task), that's the one that's linked
                #[tokio::test(flavor = "multi_thread")]
                async fn links_selected_deps_with_matching_args_when_skipping_deps() {
                    for ids in [
                        ["with-args", "shared", "with-cleanup"],
                        ["with-cleanup", "shared", "with-args"],
                    ] {
                        let (_, graph) = build_graph(&ids, true).await;

                        assert_eq!(
                            map_task_edges(&graph),
                            [
                                edge("proj:shared --force", "proj:with-cleanup", "cleanup"),
                                edge("proj:with-args", "proj:shared --force", "required"),
                            ],
                            "{ids:?}"
                        );
                    }
                }

                #[tokio::test(flavor = "multi_thread")]
                async fn ignores_deps_not_in_the_graph_when_skipping_deps() {
                    for ids in [["with-args", "without-args"], ["without-args", "with-args"]] {
                        let (context, graph) = build_graph(&ids, true).await;

                        assert!(map_task_edges(&graph).is_empty(), "{ids:?}");
                        assert_eq!(
                            context.ignored_dependencies,
                            FxHashMap::from_iter([
                                (
                                    target("proj:with-args"),
                                    FxHashSet::from_iter([target("proj:shared")])
                                ),
                                (
                                    target("proj:without-args"),
                                    FxHashSet::from_iter([target("proj:shared")])
                                ),
                            ]),
                            "{ids:?}"
                        );
                    }
                }
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

                builder.set_requirements(RunRequirements {
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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
                    Depth::NONE,
                    |affected| {
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder.set_requirements(RunRequirements {
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_direct_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder.set_requirements(RunRequirements {
                    dependents: Depth::DIRECT,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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
                    Depth::NONE,
                    |affected| {
                        affected
                            .mark_task_affected(&task, AffectedBy::AlwaysAffected)
                            .unwrap();
                        affected
                            .mark_task_affected(&task_b, AffectedBy::AlwaysAffected)
                            .unwrap();
                    },
                );

                builder.set_requirements(RunRequirements {
                    dependents: Depth::DIRECT,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_deep_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain3").unwrap();

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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
                    Depth::NONE,
                    |affected| {
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

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn expands_dependents_when_task_is_already_a_dependency() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let parent = wg.get_task_from_project("deps", "parent1").unwrap();
                let task = wg.get_task_from_project("deps", "base").unwrap();

                // First insert the task as a dependency of another target
                builder.set_requirements(RunRequirements::default());
                builder.run_task(parent.clone());

                // Then run it as an explicit target with dependents
                builder.set_requirements(RunRequirements {
                    dependents: Depth::DIRECT,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());

                assert!(topo(graph).into_iter().any(|node| matches!(
                    node,
                    ActionNode::RunTask(inner) if inner.target == Target::parse("deps:parent2").unwrap()
                )));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_set_limit_depth() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("deps", "chain5").unwrap();

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Limit(2),
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                // `chain2` is 3 hops away
                assert_eq!(
                    extract_run_task_targets(graph),
                    ["deps:chain3", "deps:chain4", "deps:chain5"]
                );

                // Dependents are primary targets as well
                let mut primary_targets = context
                    .primary_targets
                    .into_iter()
                    .map(|target| target.to_string())
                    .collect::<Vec<_>>();
                primary_targets.sort();

                assert_eq!(
                    primary_targets,
                    ["deps:chain3", "deps:chain4", "deps:chain5"]
                );
            }

            // Dependents are added to the candidates before they're partitioned,
            // so that they're spread across jobs like the requested targets
            #[tokio::test(flavor = "multi_thread")]
            async fn partitions_after_expanding_dependents() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());
                let mut targets = vec![];

                for job in 0..2 {
                    let mut builder = container
                        .create_builder(container.create_workspace_graph().await)
                        .await;

                    builder.set_requirements(RunRequirements {
                        dependents: Depth::Deep,
                        job: Some(job),
                        job_total: Some(2),
                        ..RunRequirements::default()
                    });
                    let request =
                        builder.run_tasks(vec![TargetLocator::parse("deps:chain5").unwrap()]);
                    let partition = builder
                        .build()
                        .await
                        .unwrap()
                        .report
                        .take_partition(request)
                        .unwrap();

                    assert_eq!(partition.size, Some(3));

                    let mut job_targets = partition
                        .targets
                        .into_iter()
                        .map(|(_, target)| target.to_string())
                        .collect::<Vec<_>>();
                    job_targets.sort();

                    targets.push(job_targets);
                }

                assert_eq!(
                    targets,
                    [
                        vec!["deps:chain3", "deps:chain4", "deps:chain5"],
                        vec!["deps:chain1", "deps:chain2"],
                    ]
                );
            }

            // Dependents are expanded from the selected tasks, and not from the
            // tasks that are inserted, so they're expanded when a selected task
            // isn't inserted itself (it isn't ran in CI, or is persistent)
            #[tokio::test(flavor = "multi_thread")]
            async fn expands_dependents_of_tasks_that_arent_inserted() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.set_requirements(RunRequirements {
                    ci: true,
                    ci_check: true,
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_tasks(vec![TargetLocator::parse("ci:ci2-dependency").unwrap()]);

                let context = builder.build().await.unwrap().context;

                // Both are reached, but aren't ran in CI either
                assert_eq!(
                    context.get_target_states(),
                    FxHashMap::from_iter([
                        (
                            Target::parse("ci:ci2-dependency").unwrap(),
                            TargetState::Passthrough
                        ),
                        (
                            Target::parse("ci:ci2-dependent").unwrap(),
                            TargetState::Passthrough
                        ),
                        (
                            Target::parse("ci:ci3-dependent").unwrap(),
                            TargetState::Passthrough
                        ),
                    ])
                );
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

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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

                builder.set_requirements(RunRequirements {
                    dependents: Depth::DIRECT,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

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
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                };

                // Inserts `c` as a dependency only, with no dependents expanded
                let task_b = wg.get_task_from_project("b", "task").unwrap();

                builder.set_requirements(reqs.clone());
                builder.run_task(task_b.clone());

                // Then runs `c` as an explicit target, which must expand them
                let task_c = wg.get_task_from_project("c", "task").unwrap();

                builder.set_requirements(reqs.clone());
                builder.run_task(task_c.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_eq!(
                    extract_run_task_targets(graph),
                    ["a:task", "b:task", "c:task", "d:task", "e:task"]
                );
            }

            // `c:deploy` (core) depends on `b:deploy` (edge), which depends on
            // `a:deploy` (core)
            async fn run_deploy_with_query(
                locator: &str,
                query: Option<&str>,
                changed_file: Option<&str>,
            ) -> (ActionContext, Vec<String>) {
                let sandbox = create_sandbox("graph-depth");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                if let Some(query) = query {
                    builder.set_query(query).unwrap();
                }

                if let Some(file) = changed_file {
                    builder.set_changed_files(FxHashSet::from_iter([
                        WorkspaceRelativePathBuf::from(file),
                    ]));
                    builder.track_affected(Depth::Deep, false);
                }

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_tasks(vec![TargetLocator::parse(locator).unwrap()]);

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                (context, extract_run_task_targets(graph))
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn expands_all_dependents_without_query() {
                assert_eq!(
                    run_deploy_with_query("a:deploy", None, None).await.1,
                    ["a:deploy", "b:deploy", "c:deploy"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn query_bounds_dependents() {
                // `b` doesn't match, so isn't added, and doesn't act as a bridge
                // to `c`, which does match
                assert_eq!(
                    run_deploy_with_query("a:deploy", Some("projectTag=core"), None)
                        .await
                        .1,
                    ["a:deploy"]
                );
                assert_eq!(
                    run_deploy_with_query("a:deploy", Some("projectTag=[core, edge]"), None)
                        .await
                        .1,
                    ["a:deploy", "b:deploy", "c:deploy"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn query_bounds_dependents_of_single_task() {
                let sandbox = create_sandbox("graph-depth");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.set_query("projectTag=edge").unwrap();

                let task = wg.get_task_from_project("a", "deploy").unwrap();

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                // `a` doesn't match, but was explicitly selected
                assert_eq!(extract_run_task_targets(graph), ["a:deploy", "b:deploy"]);
            }

            // A task that isn't in the task graph (like one constructed for the
            // request) has no dependents in it
            #[tokio::test(flavor = "multi_thread")]
            async fn expands_no_dependents_of_tasks_outside_the_graph() {
                let sandbox = create_sandbox("graph-depth");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..RunRequirements::default()
                });
                builder.run_task(create_task("a", "constructed"));

                let graph = builder.build().await.unwrap().graph;

                assert_eq!(extract_run_task_targets(graph), ["a:constructed"]);
            }

            // Dependencies are always inserted, so the query never bounds them.
            // Configured `:task` dependencies are rejected when tasks are built,
            // so the task is constructed manually
            #[tokio::test(flavor = "multi_thread")]
            async fn query_doesnt_bound_dependencies() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.set_query("project=client").unwrap();

                let mut task = create_task("misc", "all-builds");

                task.deps.push(TaskDependencyConfig {
                    target: Target::parse(":build").unwrap(),
                    ..TaskDependencyConfig::default()
                });

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;

                assert_eq!(
                    extract_run_task_targets(graph),
                    [
                        "base:build",
                        "client:build",
                        "common:build",
                        "misc:all-builds",
                        "server:build"
                    ]
                );
            }

            // Internal tasks only run as a dependency of another task, so they're
            // never added as dependents, but they're walked through, so that the
            // dependents beyond them are
            async fn run_internal_chain(dependents: Depth) -> (Vec<String>, Vec<String>) {
                let sandbox = create_sandbox("tasks");

                sandbox.create_file(
                    "internal-chain/moon.yml",
                    [
                        "tasks:",
                        "  source:",
                        "    command: 'source'",
                        "  prepare:",
                        "    command: 'prepare'",
                        "    deps: ['source']",
                        "    options:",
                        "      internal: true",
                        "  build:",
                        "    command: 'build'",
                        "    deps: ['prepare']",
                    ]
                    .join("\n"),
                );

                let mut container = ActionGraphContainer::new(sandbox.path());
                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.set_requirements(RunRequirements {
                    dependents,
                    ..RunRequirements::default()
                });
                builder.run_tasks(vec![TargetLocator::parse("internal-chain:source").unwrap()]);

                let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

                let mut primary_targets = context
                    .primary_targets
                    .into_iter()
                    .map(|target| target.to_string())
                    .collect::<Vec<_>>();
                primary_targets.sort();

                (primary_targets, extract_run_task_targets(graph))
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn walks_through_internal_dependents_without_adding_them() {
                let (primary_targets, targets) = run_internal_chain(Depth::Deep).await;

                assert_eq!(
                    primary_targets,
                    ["internal-chain:build", "internal-chain:source"]
                );

                // Still ran, as a dependency of `build`
                assert_eq!(
                    targets,
                    [
                        "internal-chain:build",
                        "internal-chain:prepare",
                        "internal-chain:source"
                    ]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_add_internal_direct_dependents() {
                let (primary_targets, targets) = run_internal_chain(Depth::DIRECT).await;

                assert_eq!(primary_targets, ["internal-chain:source"]);
                assert_eq!(targets, ["internal-chain:source"]);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn query_doesnt_bound_explicit_targets() {
                // `b` doesn't match, but was explicitly selected, and `a` is
                // its dependency
                assert_eq!(
                    run_deploy_with_query("b:deploy", Some("projectTag=core"), None)
                        .await
                        .1,
                    ["a:deploy", "b:deploy", "c:deploy"]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn query_doesnt_bound_affected_propagation() {
                let (context, targets) =
                    run_deploy_with_query("a:deploy", Some("projectTag=core"), Some("a/src.txt"))
                        .await;

                assert_eq!(targets, ["a:deploy"]);

                // `b` and `c` are still affected through their dependencies
                let affected = context.affected.unwrap();

                assert!(affected.is_task_affected(&Target::parse("b:deploy").unwrap()));
                assert!(affected.is_task_affected(&Target::parse("c:deploy").unwrap()));
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements {
                dependents: Depth::Deep,
                ..RunRequirements::default()
            });
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_snapshot!(graph.to_dot());
            // Dependents are primary targets as well
            let mut primary_targets = context.primary_targets.into_iter().collect::<Vec<_>>();
            primary_targets.sort();

            assert_eq!(
                primary_targets,
                [
                    Target::parse("deps-external:external").unwrap(),
                    Target::parse("deps:base").unwrap(),
                    Target::parse("deps:parent1").unwrap(),
                    Target::parse("deps:parent2").unwrap()
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_dependents_for_ci() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("deps", "base").unwrap();

            builder.set_requirements(RunRequirements {
                ci: true,
                ci_check: true,
                dependents: Depth::Deep,
                ..RunRequirements::default()
            });
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_snapshot!(graph.to_dot());
            // Dependents are primary targets as well, while `parent2` isn't ran in CI
            let mut primary_targets = context.primary_targets.into_iter().collect::<Vec<_>>();
            primary_targets.sort();

            assert_eq!(
                primary_targets,
                [
                    Target::parse("deps-external:external").unwrap(),
                    Target::parse("deps:base").unwrap(),
                    Target::parse("deps:parent1").unwrap()
                ]
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
                builder.set_requirements(RunRequirements::default());
                builder.run_task(Arc::clone(task));
            }

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [Target::parse("app:build").unwrap()]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn serial_deps_arent_ordered_after_persistent_deps() {
            let sandbox = create_sandbox("serial-persistent");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            // dev => [first-build, server, last-build] (serial), where server is
            // persistent. A persistent action is marked as completed the moment
            // it is dispatched, so ordering last-build after it would run it
            // immediately anyways. Instead, the snapshot must contain a
            // `last-build -> first-build` edge, and no `last-build -> server`
            let task = wg.get_task_from_project("proj", "dev").unwrap();

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
        }

        // dev => [first-build, server, live] (serial), where live is persistent
        // and depends on codegen. In live's subtree, live is ordered after
        // server (the previous dep), and codegen after first-build (the previous
        // dep that completes), so the link orders tasks against both, which
        // checks each edge for a cycle on its own
        #[tokio::test(flavor = "multi_thread")]
        async fn serial_subtree_orders_against_both_previous_deps() {
            let sandbox = create_sandbox("serial-persistent");

            sandbox.create_file(
                "proj/moon.yml",
                [
                    "tasks:",
                    "  first-build:",
                    "    command: 'build'",
                    "  codegen:",
                    "    command: 'codegen'",
                    "  server:",
                    "    command: 'serve'",
                    "    options:",
                    "      persistent: true",
                    "  live:",
                    "    command: 'live'",
                    "    deps: ['~:codegen']",
                    "    options:",
                    "      persistent: true",
                    "  dev:",
                    "    deps: ['~:first-build', '~:server', '~:live']",
                    "    inputs: []",
                    "    options:",
                    "      persistent: true",
                    "      runDepsInParallel: false",
                ]
                .join("\n"),
            );

            let mut container = ActionGraphContainer::new(sandbox.path());
            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            builder.set_requirements(RunRequirements::default());
            builder.run_task(wg.get_task_from_project("proj", "dev").unwrap());

            let graph = builder.build().await.unwrap().graph;
            let inner = graph.get_inner_graph();
            let mut edges = inner
                .edge_indices()
                .filter_map(|edge| {
                    let (source, target) = inner.edge_endpoints(edge).unwrap();

                    match (&inner[source], &inner[target]) {
                        (ActionNode::RunTask(source), ActionNode::RunTask(target)) => {
                            Some(format!(
                                "{} -> {}",
                                source.target.get_task_id().unwrap(),
                                target.target.get_task_id().unwrap()
                            ))
                        }
                        _ => None,
                    }
                })
                .collect::<Vec<_>>();

            edges.sort();

            assert_eq!(
                edges,
                [
                    "codegen -> first-build",
                    "dev -> first-build",
                    "dev -> live",
                    "dev -> server",
                    "live -> codegen",
                    "live -> server",
                    "server -> first-build",
                ]
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

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());
            }

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
        }

        // The serial order is a preference, while a dependency is a requirement,
        // so when the configured order contradicts a dependency between the tasks,
        // the dependency wins. The ordering edge is skipped when the dependency
        // was linked first, but must also be removed when it's linked last, which
        // happens when a cleanup is linked after the order. Either way, the graph
        // must not depend on the order that the tasks are inserted in
        mod serial_conflicts {
            use super::*;

            fn map_task_edges(graph: &ActionGraph) -> Vec<(String, String, String)> {
                let inner = graph.get_inner_graph();

                let mut edges = inner
                    .edge_indices()
                    .filter_map(|edge| {
                        let (source, target) = inner.edge_endpoints(edge).unwrap();

                        match (
                            graph.get_node_from_index(&source).unwrap(),
                            graph.get_node_from_index(&target).unwrap(),
                        ) {
                            (ActionNode::RunTask(source), ActionNode::RunTask(target)) => Some((
                                source.target.to_string(),
                                target.target.to_string(),
                                inner.edge_weight(edge).unwrap().to_string(),
                            )),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>();

                edges.sort();
                edges
            }

            fn edge(from: &str, to: &str, type_of: &str) -> (String, String, String) {
                (
                    format!("proj:{from}"),
                    format!("proj:{to}"),
                    type_of.to_owned(),
                )
            }

            async fn build_graph(ids: &[&str]) -> ActionGraph {
                let sandbox = create_sandbox("serial-conflict");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let reqs = RunRequirements::default();

                builder.set_requirements(reqs);

                for id in ids {
                    let task = wg.get_task_from_project("proj", id).unwrap();

                    builder.run_task(task.clone());
                }

                builder
                    .build()
                    .await
                    .unwrap_or_else(|error| panic!("{ids:?}: {error}"))
                    .graph
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn dependency_wins_when_linked_after_the_serial_order() {
                let expected = [
                    edge("first", "second", "required"),
                    edge("ordered", "first", "required"),
                    edge("ordered", "second", "required"),
                ];

                for ids in [["ordered", "first"], ["first", "ordered"]] {
                    assert_eq!(
                        map_task_edges(&build_graph(&ids).await),
                        expected,
                        "{ids:?}"
                    );
                }
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn wait_dependency_wins_when_linked_after_the_serial_order() {
                let expected = [
                    edge("first-wait", "second", "wait"),
                    edge("ordered-wait", "first-wait", "required"),
                    edge("ordered-wait", "second", "required"),
                ];

                for ids in [
                    ["ordered-wait", "first-wait"],
                    ["first-wait", "ordered-wait"],
                ] {
                    assert_eq!(
                        map_task_edges(&build_graph(&ids).await),
                        expected,
                        "{ids:?}"
                    );
                }
            }

            // `head` requires `tail`, so `tail` (and its own dependency) can no
            // longer run after `middle`, while `middle` still runs after `head`
            #[tokio::test(flavor = "multi_thread")]
            async fn only_removes_the_order_that_conflicts() {
                let expected = [
                    edge("head", "tail", "required"),
                    edge("middle", "head", "required"),
                    edge("ordered-three", "head", "required"),
                    edge("ordered-three", "middle", "required"),
                    edge("ordered-three", "tail", "required"),
                    edge("tail", "tail-dep", "required"),
                ];

                assert_eq!(
                    map_task_edges(&build_graph(&["tail", "ordered-three", "head"]).await),
                    expected
                );
            }

            // `c` requires `b`, which matches the order, so the ordering edge
            // between them is a dependency from then on, and must not be removed
            // when `a` requires `c`, which contradicts the order
            #[tokio::test(flavor = "multi_thread")]
            async fn keeps_the_order_that_became_a_dependency() {
                let expected = [
                    edge("a", "c", "required"),
                    edge("c", "b", "required"),
                    edge("ordered-chain", "a", "required"),
                    edge("ordered-chain", "b", "required"),
                    edge("ordered-chain", "c", "required"),
                ];

                for ids in [
                    ["ordered-chain", "c", "a"],
                    ["ordered-chain", "a", "c"],
                    ["a", "c", "ordered-chain"],
                ] {
                    assert_eq!(
                        map_task_edges(&build_graph(&ids).await),
                        expected,
                        "{ids:?}"
                    );
                }
            }

            // The orders of `nested` and `inner` contradict the dependencies of
            // `inner` and `one`, and must all be resolved in favor of them
            #[tokio::test(flavor = "multi_thread")]
            async fn removes_orders_one_after_the_other() {
                let expected = [
                    edge("inner", "one", "required"),
                    edge("inner", "two", "required"),
                    edge("nested", "inner", "required"),
                    edge("nested", "one", "required"),
                    edge("one", "two", "required"),
                ];

                for ids in [["nested", "inner", "one"], ["one", "inner", "nested"]] {
                    assert_eq!(
                        map_task_edges(&build_graph(&ids).await),
                        expected,
                        "{ids:?}"
                    );
                }
            }

            // A cleanup is linked from the cleanup to the task (it runs after it),
            // which may also contradict an order that was linked before it
            #[tokio::test(flavor = "multi_thread")]
            async fn cleanup_wins_when_linked_after_the_serial_order() {
                let expected = [
                    edge("cleaned", "setup", "required"),
                    edge("sequence", "setup", "required"),
                    edge("sequence", "teardown", "required"),
                    edge("teardown", "cleaned", "cleanup"),
                ];

                for ids in [["cleaned", "sequence"], ["sequence", "cleaned"]] {
                    assert_eq!(
                        map_task_edges(&build_graph(&ids).await),
                        expected,
                        "{ids:?}"
                    );
                }
            }

            // Only orders are removed, so tasks that
            // depend on each other are still an error
            #[tokio::test(flavor = "multi_thread")]
            async fn errors_when_dependencies_cycle() {
                let sandbox = create_sandbox("serial-conflict");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                // first -> second, and then second -> first, which is applied
                // here, as it would cycle the task graph if it was configured
                let mut task = wg
                    .get_task_from_project("proj", "second")
                    .unwrap()
                    .as_ref()
                    .to_owned();

                task.deps.push(TaskDependencyConfig {
                    target: Target::parse("proj:first").unwrap(),
                    ..TaskDependencyConfig::default()
                });

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());

                let error = builder.build().await.err().unwrap();

                assert!(
                    error.to_string().contains(
                        "adding a relationship from action RunTask(proj:second) to RunTask(proj:first) would introduce a cycle"
                    ),
                    "{error}"
                );
            }
        }
    }

    mod dep_types {
        use super::*;

        fn find_task_index(graph: &ActionGraph, target: &str) -> NodeIndex {
            let inner = graph.get_inner_graph();

            inner
                .node_indices()
                .find(|index| {
                    matches!(&inner[*index], ActionNode::RunTask(node) if node.target.as_str() == target)
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_snapshot!(graph.to_dot());

            // Tracked, as it must always run its command
            assert_eq!(
                map_targets(context.cleanup_targets.into_iter().collect()),
                ["proj:teardown"]
            );
            assert!(context.wait_targets.is_empty());

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_snapshot!(graph.to_dot());

            // Tracked, as it must always run its command
            assert_eq!(
                map_targets(context.wait_targets.into_iter().collect()),
                ["proj:server"]
            );
            assert!(context.cleanup_targets.is_empty());

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());

            let edges = map_edges(&graph);
            let mut task_edges = edges
                .iter()
                .filter(|(source, target, _)| {
                    source.contains("Task(proj:") && target.contains("Task(proj:")
                })
                .cloned()
                .collect::<Vec<_>>();

            task_edges.sort();

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
                        "RunPersistentTask(proj:server)".into(),
                        "wait".into()
                    ),
                    (
                        "RunTask(proj:serial)".into(),
                        "RunTask(proj:a)".into(),
                        "required".into()
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
        async fn doesnt_chain_serial_deps_after_non_persistent_wait_deps() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "serial-wait").unwrap();

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

            let edges = map_edges(&graph);
            let mut task_edges = edges
                .iter()
                .filter(|(source, target, _)| {
                    source.contains("Task(proj:") && target.contains("Task(proj:")
                })
                .cloned()
                .collect::<Vec<_>>();

            task_edges.sort();

            // Unlike a persistent dep, a non-persistent wait dep completes,
            // but it's not waited on, so b is still chained after a (not c)
            assert_eq!(
                task_edges,
                vec![
                    (
                        "RunTask(proj:b)".into(),
                        "RunTask(proj:a)".into(),
                        "required".into()
                    ),
                    (
                        "RunTask(proj:serial-wait)".into(),
                        "RunTask(proj:a)".into(),
                        "required".into()
                    ),
                    (
                        "RunTask(proj:serial-wait)".into(),
                        "RunTask(proj:b)".into(),
                        "required".into()
                    ),
                    (
                        "RunTask(proj:serial-wait)".into(),
                        "RunTask(proj:c)".into(),
                        "wait".into()
                    ),
                ]
            );
        }

        fn find_task_args(graph: &ActionGraph, target: &str) -> Vec<Vec<String>> {
            graph
                .get_nodes()
                .into_iter()
                .filter_map(|node| match node {
                    ActionNode::RunTask(inner) if inner.target.as_str() == target => {
                        Some(inner.args.clone())
                    }
                    _ => None,
                })
                .collect()
        }

        // Cleanups are dependents in the task graph (they run after the task),
        // but must not be ran again as dependents, as that would create another
        // action (without their args) that isn't ordered after the task
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_cleanups_again_as_dependents() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "args-parent").unwrap();

            builder.set_requirements(RunRequirements {
                dependents: Depth::DIRECT,
                ..Default::default()
            });
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_eq!(
                find_task_args(&graph, "proj:teardown"),
                vec![vec!["--force".to_string()]]
            );
            assert!(map_edges(&graph).contains(&(
                "RunTask(proj:teardown)".into(),
                "RunTask(proj:args-parent)".into(),
                "cleanup".into()
            )));
        }

        // Cleanups always run after the task, even when its dependencies
        // (which run before it) are skipped
        #[tokio::test(flavor = "multi_thread")]
        async fn runs_cleanups_when_skipping_dependencies() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "args-parent").unwrap();

            builder.set_requirements(RunRequirements {
                no_dependencies: true,
                dependents: Depth::DIRECT,
                ..Default::default()
            });
            builder.run_task(task.clone());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            // The cleanup is linked, so only the others are ignored
            assert_eq!(
                map_targets(
                    context.ignored_dependencies[&Target::parse("proj:args-parent").unwrap()]
                        .iter()
                        .cloned()
                        .collect()
                ),
                ["proj:setup"]
            );

            assert!(find_task_args(&graph, "proj:setup").is_empty());
            assert_eq!(
                find_task_args(&graph, "proj:teardown"),
                vec![vec!["--force".to_string()]]
            );
            assert!(map_edges(&graph).contains(&(
                "RunTask(proj:teardown)".into(),
                "RunTask(proj:args-parent)".into(),
                "cleanup".into()
            )));
        }

        // Tasks may relate to each other in multiple ways, and every
        // relationship must be kept, as the pipeline honors all of them
        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_a_cleanup_that_also_requires_its_task() {
            for id in ["both-parent", "both-cleanup"] {
                let sandbox = create_sandbox("dep-types");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                let task = wg.get_task_from_project("proj", id).unwrap();

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());

                let graph = builder.build().await.unwrap().graph;
                let edges = map_edges(&graph);

                assert!(
                    edges.contains(&(
                        "RunTask(proj:both-cleanup)".into(),
                        "RunTask(proj:both-parent)".into(),
                        "cleanup".into()
                    )),
                    "missing cleanup edge when running {id}"
                );
                assert!(
                    edges.contains(&(
                        "RunTask(proj:both-cleanup)".into(),
                        "RunTask(proj:both-parent)".into(),
                        "required".into()
                    )),
                    "missing required edge when running {id}"
                );
                assert!(graph.is_cleanup_index(&find_task_index(&graph, "proj:both-cleanup")));
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_serial_order_for_a_wait_dependency() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg
                .get_task_from_project("proj", "serial-inner-wait")
                .unwrap();

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;
            let edges = map_edges(&graph);

            // Waits on the first to start, but serially, must run after it completes
            assert!(edges.contains(&(
                "RunTask(proj:s-second)".into(),
                "RunTask(proj:s-first)".into(),
                "wait".into()
            )));
            assert!(edges.contains(&(
                "RunTask(proj:s-second)".into(),
                "RunTask(proj:s-first)".into(),
                "required".into()
            )));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn shares_a_cleanup_between_tasks() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            for id in ["base", "other"] {
                let task = wg.get_task_from_project("proj", id).unwrap();

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());
            }

            let graph = builder.build().await.unwrap().graph;

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

        // Maps are equal regardless of the order of their keys,
        // so the same cleanup must not be inserted twice
        #[tokio::test(flavor = "multi_thread")]
        async fn shares_a_cleanup_when_env_is_ordered_differently() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            for id in ["env-first", "env-second"] {
                let task = wg.get_task_from_project("proj", id).unwrap();

                builder.set_requirements(RunRequirements::default());
                builder.run_task(task.clone());
            }

            let graph = builder.build().await.unwrap().graph;
            let edges = map_edges(&graph);

            assert!(edges.contains(&(
                "RunTask(proj:teardown)".into(),
                "RunTask(proj:env-first)".into(),
                "cleanup".into()
            )));
            assert!(edges.contains(&(
                "RunTask(proj:teardown)".into(),
                "RunTask(proj:env-second)".into(),
                "cleanup".into()
            )));
            assert_eq!(graph.get_cleanup_indices().len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_expand_cleanups_as_dependents() {
            let sandbox = create_sandbox("dep-types");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let task = wg.get_task_from_project("proj", "base").unwrap();

            builder.set_requirements(RunRequirements {
                dependents: Depth::DIRECT,
                ..RunRequirements::default()
            });
            builder.run_task(task.clone());

            let graph = builder.build().await.unwrap().graph;

            // The cleanup task is a "dependent" of the task it cleans up, but
            // cleanup edges aren't followed when expanding dependents, as it's
            // inserted as a dependency, so it must not be duplicated, nor gain
            // a second edge
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
                    .get_nodes()
                    .into_iter()
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task(task.clone());

            let error = builder.build().await.err().unwrap();

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
                vec![
                    "proj:args-parent",
                    "proj:base",
                    "proj:env-first",
                    "proj:env-second",
                    "proj:other",
                    "proj:serial"
                ]
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("^:build").unwrap());

            builder.build().await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Self scope (~:) is not supported in run contexts.")]
        async fn errors_on_self_scope() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("~:build").unwrap());

            builder.build().await.unwrap();
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("unknown:build").unwrap());

            builder.build().await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task unknown for project server.")]
        async fn errors_for_unknown_project_task() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("server:unknown").unwrap());

            builder.build().await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unknown task internal for project common.")]
        async fn errors_for_internal_task_when_explicit() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("common:internal").unwrap());

            builder.build().await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_all() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse(":build").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                context.primary_targets.into_iter().collect::<Vec<_>>(),
                [
                    Target::parse("base:build").unwrap(),
                    Target::parse("client:build").unwrap(),
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse(":build").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse(":unknown").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse(":internal").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("client:lint").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("misc:requiresInternal").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("#frontend:lint").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("#unknown:lint").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("#frontend:internal").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("client:#quality").unwrap());

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse(":#quality").unwrap());

            let graph = builder.build().await.unwrap().graph;
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("#frontend:#quality").unwrap());

            let graph = builder.build().await.unwrap().graph;
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse("server:#quality").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target(Target::parse(":#unknown-tag").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::Qualified(
                Target::parse("common:internal").unwrap(),
            ));

            builder.build().await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_target() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::Qualified(
                Target::parse("server:build").unwrap(),
            ));

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::parse(":*-dependency").unwrap());

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::parse(":{a,c}").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::parse("#front*:build").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::parse(":#qual*").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::parse("#front*:#qual*").unwrap());

            let context = builder.build().await.unwrap().context;

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::parse(":#unknown-*").unwrap());

            let context = builder.build().await.unwrap().context;

            assert!(context.primary_targets.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_by_project_glob() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements::default());
            builder
                .run_task_by_target_locator(TargetLocator::parse("c{lient,ommon}:test").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::parse("{foo,bar}:task-*").unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::Qualified(
                Target::parse("misc:requiresInternal").unwrap(),
            ));

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::DefaultProject(Id::raw("build")));

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::DefaultProject(Id::raw("build")));

            builder.build().await.unwrap();
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

            builder.set_requirements(RunRequirements::default());
            builder.run_task_by_target_locator(TargetLocator::DefaultProject(Id::raw("build")));

            builder.build().await.unwrap();
        }
    }

    mod run_tasks {
        use super::*;

        // Selected (requested) targets first, in the order of the locators,
        // so that the "Resolved targets" output is stable
        #[tokio::test(flavor = "multi_thread")]
        async fn partition_targets_are_in_selection_order() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements {
                job: Some(0),
                job_total: Some(1),
                ..Default::default()
            });

            let request = builder.run_tasks(
                ["partition:task-3", "partition:task-1", "partition:task-2"]
                    .into_iter()
                    .map(|locator| TargetLocator::parse(locator).unwrap())
                    .collect::<Vec<_>>(),
            );

            let partition = builder
                .build()
                .await
                .unwrap()
                .report
                .take_partition(request)
                .unwrap();

            assert_eq!(
                partition
                    .targets
                    .into_iter()
                    .map(|(_, target)| target.to_string())
                    .collect::<Vec<_>>(),
                ["partition:task-3", "partition:task-1", "partition:task-2"]
            );
        }

        // Jobs are validated before any task is selected
        #[tokio::test(flavor = "multi_thread")]
        async fn errors_for_zero_job_total() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements {
                job: Some(0),
                job_total: Some(0),
                ..Default::default()
            });
            builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            let error = builder.build().await.err().unwrap().to_string();

            assert!(
                error.contains("Received 0 but the total number of jobs is 0"),
                "{error}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_for_job_index_out_of_range() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements {
                job: Some(3),
                job_total: Some(3),
                ..Default::default()
            });
            builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            let error = builder.build().await.err().unwrap().to_string();

            assert!(
                error.contains("Received 3 but the total number of jobs is 3"),
                "{error}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_when_tracking_affected_without_changed_files() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.track_affected(Depth::Deep, false);
            builder.set_requirements(RunRequirements::default());
            builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            let error = builder.build().await.err().unwrap().to_string();

            assert!(
                error.contains("the changed files have not been set"),
                "{error}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_partition_if_no_job() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());

            // 0
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements::default());
            builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            let context = builder.build().await.unwrap().context;

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

            builder.set_requirements(RunRequirements {
                job: Some(0),
                job_total: Some(3),
                ..Default::default()
            });
            builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_eq!(context.primary_targets.len(), 4);
            assert_snapshot!(graph.to_dot());

            // 1
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements {
                job: Some(1),
                job_total: Some(3),
                ..Default::default()
            });
            builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_eq!(context.primary_targets.len(), 4);
            assert_snapshot!(graph.to_dot());

            // 2
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.set_requirements(RunRequirements {
                job: Some(2),
                job_total: Some(3),
                ..Default::default()
            });
            builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            assert_eq!(context.primary_targets.len(), 2);
            assert_snapshot!(graph.to_dot());
        }

        async fn run_job(
            container: &mut ActionGraphContainer,
            wg: &Arc<WorkspaceGraph>,
            job: usize,
            job_total: usize,
        ) -> RunPartition {
            let mut builder = container.create_builder(wg.clone()).await;

            builder.set_requirements(RunRequirements {
                job: Some(job),
                job_total: Some(job_total),
                ..Default::default()
            });

            let request =
                builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);

            builder
                .build()
                .await
                .unwrap()
                .report
                .take_partition(request)
                .unwrap()
        }

        // The last jobs can start past the end of the candidates (10 over 6
        // jobs leaves the last one empty), and must then run nothing
        #[tokio::test(flavor = "multi_thread")]
        async fn partitions_every_candidate_into_one_job() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let wg = container.create_workspace_graph().await;

            let mut expected = (1..=10)
                .map(|i| format!("partition:task-{i}"))
                .collect::<Vec<_>>();
            expected.sort();

            for job_total in [1, 3, 4, 6, 9, 10, 12] {
                let mut targets = vec![];

                for job in 0..job_total {
                    let partition = run_job(&mut container, &wg, job, job_total).await;
                    let size = partition.size.unwrap();

                    assert!(
                        partition.targets.len() <= size,
                        "job {job} of {job_total} has {} targets, more than {size}",
                        partition.targets.len()
                    );

                    targets.extend(
                        partition
                            .targets
                            .into_iter()
                            .map(|(_, target)| target.to_string()),
                    );
                }

                targets.sort();

                assert_eq!(targets, expected, "{job_total} jobs");
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn partitions_more_jobs_than_candidates() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let wg = container.create_workspace_graph().await;

            for job in 0..12 {
                let partition = run_job(&mut container, &wg, job, 12).await;

                assert_eq!(partition.size, Some(1));
                assert_eq!(partition.targets.len(), usize::from(job < 10), "job {job}");
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn partitions_no_candidates() {
            let sandbox = create_sandbox("tasks");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let wg = container.create_workspace_graph().await;

            for job in 0..3 {
                let mut builder = container.create_builder(wg.clone()).await;

                // Nothing changed, so nothing is affected
                builder.set_changed_files(FxHashSet::default());
                builder.track_affected(Depth::Deep, false);
                builder.set_requirements(RunRequirements {
                    job: Some(job),
                    job_total: Some(3),
                    ..Default::default()
                });

                let request =
                    builder.run_tasks(vec![TargetLocator::parse("partition:task-*").unwrap()]);
                let partition = builder
                    .build()
                    .await
                    .unwrap()
                    .report
                    .take_partition(request)
                    .unwrap();

                assert_eq!(partition.size, Some(0));
                assert!(partition.targets.is_empty(), "job {job}");
            }
        }
    }

    // How dependencies roots compose actions, for each kind of root, with
    // and without a project overriding the toolchain's version. The
    // `shared-root` fixture is a dependencies workspace at the root (with
    // `packages/*` members), a `standalone` package root, and `tools`, which
    // is in no root. The workspace configures version 1.2.3 for tier 3.
    mod dependencies_roots {
        use super::*;
        use starbase_sandbox::Sandbox;

        const TC: &str = "tc-tier3-setup-env";

        fn write_project(sandbox: &Sandbox, source: &str, toolchains: &str) {
            sandbox.create_file(
                format!("{source}/moon.yml"),
                format!("{toolchains}\ntasks:\n  build:\n    command: 'build'\n"),
            );
        }

        fn uses_toolchain(sandbox: &Sandbox, source: &str) {
            write_project(sandbox, source, &format!("toolchains:\n  default: '{TC}'"));
        }

        fn overrides_version(sandbox: &Sandbox, source: &str) {
            write_project(
                sandbox,
                source,
                &format!("toolchains:\n  default: '{TC}'\n  {TC}:\n    version: '1.99.0'"),
            );
        }

        // Locate roots from each project, instead of the working dir
        fn create_container(sandbox: &Sandbox, toolchain_id: &str) -> ActionGraphContainer {
            let mut container = ActionGraphContainer::new(sandbox.path());

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                cfg.plugins
                    .entry(Id::raw(toolchain_id))
                    .or_default()
                    .config
                    .insert("testLocateRoots".into(), serde_json::json!(true));
            });
            container
        }

        async fn build_tasks(container: &mut ActionGraphContainer, ids: &[&str]) -> ActionGraph {
            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg).await;

            for id in ids {
                builder.run_task_by_target(Target::parse(&format!("{id}:build")).unwrap());
            }

            builder.build().await.unwrap().graph
        }

        fn find_node(graph: &ActionGraph, label: &str) -> NodeIndex {
            graph
                .get_inner_graph()
                .node_indices()
                .find(|index| graph.get_node_from_index(index).unwrap().label() == label)
                .unwrap_or_else(|| panic!("Missing node {label}"))
        }

        fn has_node(graph: &ActionGraph, label: &str) -> bool {
            graph.get_nodes().iter().any(|node| node.label() == label)
        }

        // The dependencies of a node, without project syncs, which are
        // linked by tasks whatever their toolchains
        fn dependency_labels(graph: &ActionGraph, label: &str) -> Vec<String> {
            let mut labels = graph
                .get_inner_graph()
                .neighbors_directed(find_node(graph, label), petgraph::Direction::Outgoing)
                .map(|index| graph.get_node_from_index(&index).unwrap().label())
                .filter(|label| !label.starts_with("SyncProject"))
                .collect::<Vec<_>>();
            labels.sort();
            labels.dedup();
            labels
        }

        // Only the dependencies of the toolchain under test, as a project
        // with a `tc.lock` is also detected to use the other test toolchains
        fn toolchain_dependency_labels(graph: &ActionGraph, label: &str) -> Vec<String> {
            dependency_labels(graph, label)
                .into_iter()
                .filter(|label| label.contains(TC))
                .collect()
        }

        fn setup_toolchain(version: &str) -> String {
            format!("SetupToolchain({TC}:{version})")
        }

        // Tasks are hashed with the dependencies of their roots, so their
        // toolchains are placed even when they don't install dependencies,
        // while install requests are only placed when they do
        #[tokio::test(flavor = "multi_thread")]
        async fn places_task_toolchains_when_installs_are_disabled() {
            let sandbox = create_sandbox("shared-root");

            uses_toolchain(&sandbox, "packages/a");
            uses_toolchain(&sandbox, "packages/c");

            let mut container = create_container(&sandbox, TC);

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                cfg.plugins
                    .get_mut(&Id::raw(TC))
                    .unwrap()
                    .install_dependencies = false;
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            builder.run_task_by_target(Target::parse("a:build").unwrap());
            builder.install_dependencies(
                &create_tier_spec_with_name(TC),
                wg.get_project("c").unwrap(),
            );

            // The pipeline reads the layout from the graph
            let graph = builder.build().await.unwrap().graph;
            let layout = graph.get_layout_graph();

            assert!(
                layout
                    .binding(&Id::raw("a"), &Id::raw(TC))
                    .is_some_and(|binding| binding.root_index().is_some())
            );
            assert!(layout.binding(&Id::raw("c"), &Id::raw(TC)).is_none());
        }

        mod workspace {
            use super::*;

            // 0 -> 1 -> 2 -> 3, all with the workspace version
            #[tokio::test(flavor = "multi_thread")]
            async fn without_overrides() {
                let sandbox = create_sandbox("shared-root");

                uses_toolchain(&sandbox, "packages/a");
                uses_toolchain(&sandbox, "packages/b");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["a", "b"]).await;

                for id in ["a", "b"] {
                    assert_eq!(
                        dependency_labels(&graph, &format!("RunTask({id}:build)")),
                        vec![format!("InstallDependencies({TC})")]
                    );
                }

                assert_eq!(
                    dependency_labels(&graph, &format!("InstallDependencies({TC})")),
                    vec![format!("SetupEnvironment({TC})")]
                );
                assert_eq!(
                    dependency_labels(&graph, &format!("SetupEnvironment({TC})")),
                    vec![setup_toolchain("1.2.3"), "SyncWorkspace".into()]
                );
            }

            // 0 -> 1 -> 2 -> 3 with the workspace version, and 0 -> 4 with
            // the project's version, only for running the task
            #[tokio::test(flavor = "multi_thread")]
            async fn with_overrides() {
                let sandbox = create_sandbox("shared-root");

                overrides_version(&sandbox, "packages/a");
                uses_toolchain(&sandbox, "packages/b");
                uses_toolchain(&sandbox, "packages/c");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["a", "b", "c"]).await;

                assert_eq!(
                    dependency_labels(&graph, "RunTask(a:build)"),
                    vec![
                        format!("InstallDependencies({TC})"),
                        setup_toolchain("1.99.0")
                    ]
                );

                for id in ["b", "c"] {
                    assert_eq!(
                        dependency_labels(&graph, &format!("RunTask({id}:build)")),
                        vec![format!("InstallDependencies({TC})")]
                    );
                }

                assert_eq!(
                    dependency_labels(&graph, &format!("InstallDependencies({TC})")),
                    vec![format!("SetupEnvironment({TC})")]
                );
                assert_eq!(
                    dependency_labels(&graph, &format!("SetupEnvironment({TC})")),
                    vec![setup_toolchain("1.2.3"), "SyncWorkspace".into()]
                );
            }

            // The workspace version is setup for the root's actions, even
            // when no project in the run uses it
            #[tokio::test(flavor = "multi_thread")]
            async fn with_overrides_when_running_alone() {
                let sandbox = create_sandbox("shared-root");

                overrides_version(&sandbox, "packages/a");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["a"]).await;

                assert_eq!(
                    dependency_labels(&graph, "RunTask(a:build)"),
                    vec![
                        format!("InstallDependencies({TC})"),
                        setup_toolchain("1.99.0")
                    ]
                );
                assert_eq!(
                    dependency_labels(&graph, &format!("SetupEnvironment({TC})")),
                    vec![setup_toolchain("1.2.3"), "SyncWorkspace".into()]
                );
            }

            // The root's actions run with the root project's version, like a
            // package, so the root project needs no version of its own
            #[tokio::test(flavor = "multi_thread")]
            async fn with_an_override_in_the_root_project() {
                let sandbox = create_sandbox("shared-root");

                overrides_version(&sandbox, ".");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["root"]).await;
                let deps = dependency_labels(&graph, "RunTask(root:build)");

                assert!(deps.contains(&format!("InstallDependencies({TC})")));
                assert!(!deps.iter().any(|dep| dep.starts_with("SetupToolchain")));
                assert_eq!(
                    dependency_labels(&graph, &format!("SetupEnvironment({TC})")),
                    vec![setup_toolchain("1.99.0"), "SyncWorkspace".into()]
                );
            }

            // While members that don't override it run their tasks with the
            // workspace's version, which they setup on their own
            #[tokio::test(flavor = "multi_thread")]
            async fn with_an_override_in_the_root_project_for_members() {
                let sandbox = create_sandbox("shared-root");

                overrides_version(&sandbox, ".");
                uses_toolchain(&sandbox, "packages/b");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["b"]).await;

                assert_eq!(
                    dependency_labels(&graph, "RunTask(b:build)"),
                    vec![
                        format!("InstallDependencies({TC})"),
                        setup_toolchain("1.2.3")
                    ]
                );
                assert_eq!(
                    dependency_labels(&graph, &format!("SetupEnvironment({TC})")),
                    vec![setup_toolchain("1.99.0"), "SyncWorkspace".into()]
                );
            }

            // Like JavaScript requiring Node.js, which `a` overrides: the
            // root's toolchain setup requires the workspace version, while
            // `a` sets up its own through its own `tc-tier3` toolchain
            #[tokio::test(flavor = "multi_thread")]
            async fn with_overrides_of_required_toolchains() {
                let sandbox = create_sandbox("shared-root");

                write_project(
                    &sandbox,
                    "packages/a",
                    "toolchains:\n  default: ['tc-tier2-reqs', 'tc-tier3']\n  tc-tier3:\n    version: '4.5.6'",
                );
                write_project(
                    &sandbox,
                    "packages/b",
                    "toolchains:\n  default: ['tc-tier2-reqs', 'tc-tier3']",
                );

                let mut container = create_container(&sandbox, "tc-tier2-reqs");
                let graph = build_tasks(&mut container, &["a", "b"]).await;
                let requirements = dependency_labels(&graph, "SetupToolchain(tc-tier2-reqs)");

                assert!(requirements.contains(&"SetupToolchain(tc-tier3:1.2.3)".to_owned()));
                assert!(!requirements.contains(&"SetupToolchain(tc-tier3:4.5.6)".to_owned()));
                assert!(
                    dependency_labels(&graph, "RunTask(a:build)")
                        .contains(&"SetupToolchain(tc-tier3:4.5.6)".to_owned())
                );
            }
        }

        mod package {
            use super::*;

            // 0 -> 1 -> 2 -> 3, all with the workspace version, which is the
            // project's version
            #[tokio::test(flavor = "multi_thread")]
            async fn without_override() {
                let sandbox = create_sandbox("shared-root");

                uses_toolchain(&sandbox, "standalone");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["standalone"]).await;
                let install = format!("InstallDependencies({TC}, standalone)");
                let env = format!("SetupEnvironment({TC}, standalone)");

                assert_eq!(
                    toolchain_dependency_labels(&graph, "RunTask(standalone:build)"),
                    vec![install.clone()]
                );
                assert_eq!(dependency_labels(&graph, &install), vec![env.clone()]);
                assert_eq!(
                    dependency_labels(&graph, &env),
                    vec![setup_toolchain("1.2.3"), "SyncWorkspace".into()]
                );
            }

            // 0 -> 1 -> 2 -> 3, all with the project's version, and the
            // workspace version isn't setup at all
            #[tokio::test(flavor = "multi_thread")]
            async fn with_override() {
                let sandbox = create_sandbox("shared-root");

                overrides_version(&sandbox, "standalone");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["standalone"]).await;
                let install = format!("InstallDependencies({TC}, standalone)");
                let env = format!("SetupEnvironment({TC}, standalone)");

                assert_eq!(
                    toolchain_dependency_labels(&graph, "RunTask(standalone:build)"),
                    vec![install.clone()]
                );
                assert_eq!(dependency_labels(&graph, &install), vec![env.clone()]);
                assert_eq!(
                    dependency_labels(&graph, &env),
                    vec![setup_toolchain("1.99.0"), "SyncWorkspace".into()]
                );
                assert!(!has_node(&graph, &setup_toolchain("1.2.3")));
            }

            // `moon setup` places the workspace root, so a single package at
            // the root uses the version of the project that owns it
            #[tokio::test(flavor = "multi_thread")]
            async fn with_override_from_the_workspace_root() {
                let sandbox = create_sandbox("shared-root");

                std::fs::remove_file(sandbox.path().join("tc.root.cfg")).unwrap();
                overrides_version(&sandbox, ".");

                let mut container = create_container(&sandbox, TC);
                let wg = container.create_workspace_graph().await;
                let mut builder = container.create_builder(wg.clone()).await;

                builder.setup_toolchain_and_environment(&create_tier_spec_with_name(TC));

                let graph = builder.build().await.unwrap().graph;

                assert!(has_node(&graph, &setup_toolchain("1.2.3")));
                assert_eq!(
                    dependency_labels(&graph, &format!("SetupEnvironment({TC})")),
                    vec![setup_toolchain("1.99.0"), "SyncWorkspace".into()]
                );
                assert!(graph.get_nodes().iter().any(|node| matches!(
                    node,
                    ActionNode::SetupEnvironment(inner) if inner.project_id == Some(Id::raw("root"))
                )));
            }
        }

        mod no_deps {
            use super::*;

            // 0 -> 1, with the workspace version
            #[tokio::test(flavor = "multi_thread")]
            async fn without_override() {
                let sandbox = create_sandbox("shared-root");

                uses_toolchain(&sandbox, "tools");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["tools"]).await;

                assert_eq!(
                    dependency_labels(&graph, "RunTask(tools:build)"),
                    vec![setup_toolchain("1.2.3")]
                );
                assert!(!has_node(&graph, &format!("InstallDependencies({TC})")));
                assert!(!has_node(&graph, &format!("SetupEnvironment({TC})")));
            }

            // 0 -> 1, with the project's version
            #[tokio::test(flavor = "multi_thread")]
            async fn with_override() {
                let sandbox = create_sandbox("shared-root");

                overrides_version(&sandbox, "tools");

                let mut container = create_container(&sandbox, TC);
                let graph = build_tasks(&mut container, &["tools"]).await;

                assert_eq!(
                    dependency_labels(&graph, "RunTask(tools:build)"),
                    vec![setup_toolchain("1.99.0")]
                );
                assert!(!has_node(&graph, &setup_toolchain("1.2.3")));
                assert!(!has_node(&graph, &format!("InstallDependencies({TC})")));
            }

            // A member whose toolchain doesn't install dependencies is in no
            // root, so it's like a project outside of one
            #[tokio::test(flavor = "multi_thread")]
            async fn with_override_when_installs_are_disabled() {
                let sandbox = create_sandbox("shared-root");

                overrides_version(&sandbox, "packages/a");

                let mut container = create_container(&sandbox, TC);

                container.mocker = container.mocker.update_toolchains_config(|cfg| {
                    cfg.plugins
                        .get_mut(&Id::raw(TC))
                        .unwrap()
                        .install_dependencies = false;
                });

                let graph = build_tasks(&mut container, &["a"]).await;

                assert_eq!(
                    dependency_labels(&graph, "RunTask(a:build)"),
                    vec![setup_toolchain("1.99.0")]
                );
                assert!(!has_node(&graph, &format!("InstallDependencies({TC})")));
                assert!(!has_node(&graph, &format!("SetupEnvironment({TC})")));
            }
        }
    }

    mod setup_env {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_for_invalid_toolchain_requirement_ids() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-setup-env")) {
                    inner
                        .config
                        .insert("testEnvRequirements".into(), serde_json::json!(["bad id!"]));
                }
            });

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            builder.install_dependencies(
                &create_tier_spec_with_name("tc-tier2-setup-env"),
                wg.get_project("bar").unwrap(),
            );

            let error = builder.build().await.err().unwrap().to_string();

            assert!(
                error.contains("requires bad id!, which is not a valid toolchain ID"),
                "{error}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_if_supported() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            let index = builder.setup_environment(&spec, &project.source, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
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
            builder.setup_environment(&spec, &bar.source, bar.clone());

            let baz = wg.get_project("baz").unwrap();
            builder.setup_environment(&spec, &baz.source, baz.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
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
            let index1 = builder.setup_environment(&spec, &project.source, project.clone());
            let index2 = builder.setup_environment(&spec, &project.source, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert_eq!(output.report.nodes(index1), output.report.nodes(index2));

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
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
            let index = builder.setup_environment(&spec, &project.source, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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
            builder.setup_environment(&spec, &project.source, project.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.setup_environment(&spec, &project.source, project.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.setup_environment(&spec, &project.source, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
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
            let index = builder.setup_environment(&spec, &project.source, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

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
                        owner_project_id: None,
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
            let index = builder.setup_environment(&spec, &project.source, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

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
                        owner_project_id: None,
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
            let index = builder.setup_environment(&spec, &project.source, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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

            builder.setup_environment(&spec, &project.source, project.clone());

            let error = builder.build().await.err().unwrap();

            assert_eq!(
                error.to_string(),
                "Toolchain tc-tier2-setup-env requires the toolchain tc-unknown, but it has not been configured!"
            );
        }

        fn require_toolchains(
            mut container: ActionGraphContainer,
            id: &str,
            key: &str,
            requires: &[&str],
        ) -> ActionGraphContainer {
            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                cfg.plugins
                    .entry(Id::raw(id))
                    .or_default()
                    .config
                    .insert(key.into(), serde_json::json!(requires));
            });
            container
        }

        fn add_toolchain(mut container: ActionGraphContainer, id: &str) -> ActionGraphContainer {
            container.mocker = container.mocker.update_toolchains_config(|cfg| {
                cfg.plugins.entry(Id::raw(id)).or_default();
            });
            container
        }

        fn create_bar_env_node(id: &str) -> ActionNode {
            ActionNode::setup_environment(SetupEnvironmentNode {
                owner_project_id: None,
                project_id: Some(Id::raw("bar")),
                root: WorkspaceRelativePathBuf::from("bar"),
                toolchain_id: Id::raw(id),
            })
        }

        fn sorted_dependencies_of(graph: &ActionGraph, node: &ActionNode) -> Vec<String> {
            let mut deps = graph.dependencies_of(node);
            deps.sort();
            deps
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_up_required_toolchain_environments() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            // Like the JavaScript toolchain requiring Node.js and a package
            // manager, whose environments must be setup in its root
            container = add_toolchain(container, "tc-tier3-setup-env");
            container = require_toolchains(
                container,
                "tc-tier2-setup-env",
                "testRequirements",
                &["tc-tier3-setup-env"],
            );

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder.setup_environment(&spec, &project.source, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                sorted_dependencies_of(&graph, &create_bar_env_node("tc-tier2-setup-env")),
                vec!["SetupEnvironment(tc-tier3-setup-env, bar)", "SyncWorkspace"]
            );
            assert_eq!(
                sorted_dependencies_of(&graph, &create_bar_env_node("tc-tier3-setup-env")),
                vec!["SetupToolchain(tc-tier3-setup-env:1.2.3)", "SyncWorkspace"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_up_required_toolchain_environments_with_requirements_for_environment() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container = add_toolchain(container, "tc-tier3-setup-env");
            container = require_toolchains(
                container,
                "tc-tier2-setup-env",
                "testEnvRequirements",
                &["tc-tier3-setup-env"],
            );

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder.setup_environment(&spec, &project.source, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                sorted_dependencies_of(&graph, &create_bar_env_node("tc-tier2-setup-env")),
                vec![
                    "SetupEnvironment(tc-tier3-setup-env, bar)",
                    "SetupToolchain(tc-tier3-setup-env:1.2.3)",
                    "SyncWorkspace"
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn installs_deps_after_required_toolchain_environments() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path())
                // Plugin matches based on cwd, so the project owns the root package
                .set_working_dir(sandbox.path().join("bar"));

            container = add_toolchain(container, "tc-tier3-setup-env");
            container = require_toolchains(
                container,
                "tc-tier2-setup-env",
                "testRequirements",
                &["tc-tier3-setup-env"],
            );

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            builder.install_dependencies(&spec, project.clone());

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_proto(create_proto_version()),
                    ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_tier_spec_with_name("tc-tier3-setup-env"),
                    }),
                    create_bar_env_node("tc-tier3-setup-env"),
                    create_bar_env_node("tc-tier2-setup-env"),
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: None,
                        owner_project_id: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id,
                    })
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_link_required_toolchain_environments_that_would_cycle() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            // Toolchains may require each other, like Python and uv
            container = require_toolchains(
                container,
                "tc-tier2-setup-env",
                "testRequirements",
                &["tc-tier3-setup-env"],
            );
            container = require_toolchains(
                container,
                "tc-tier3-setup-env",
                "testRequirements",
                &["tc-tier2-setup-env"],
            );

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let project = wg.get_project("bar").unwrap();

            for id in ["tc-tier2-setup-env", "tc-tier3-setup-env"] {
                builder.setup_environment(
                    &create_tier_spec_with_name(id),
                    &project.source,
                    project.clone(),
                );
            }

            let graph = builder.build().await.unwrap().graph;

            assert!(graph.sort_topological().is_ok());
            assert_eq!(
                sorted_dependencies_of(&graph, &create_bar_env_node("tc-tier2-setup-env")),
                vec!["SetupEnvironment(tc-tier3-setup-env, bar)", "SyncWorkspace"]
            );
            assert_eq!(
                sorted_dependencies_of(&graph, &create_bar_env_node("tc-tier3-setup-env")),
                vec!["SetupToolchain(tc-tier3-setup-env:1.2.3)", "SyncWorkspace"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_required_toolchains_not_configured_if_not_for_environment() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            container = require_toolchains(
                container,
                "tc-tier2-setup-env",
                "testRequirements",
                &["tc-unknown"],
            );

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let spec = create_tier_spec_with_name("tc-tier2-setup-env");

            let project = wg.get_project("bar").unwrap();
            let index = builder.setup_environment(&spec, &project.source, project.clone());

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    create_bar_env_node("tc-tier2-setup-env"),
                ]
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
            builder.setup_environment(&spec, &bar.source, bar.clone());

            let baz = wg.get_project("baz").unwrap();
            builder.setup_environment(&spec, &baz.source, baz.clone());

            let graph = builder.build().await.unwrap().graph;

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
                        owner_project_id: None,
                        project_id: Some(Id::raw("bar")),
                        root: WorkspaceRelativePathBuf::from("bar"),
                        toolchain_id: spec.id.clone(),
                    }),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
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

            let index = builder.setup_environment_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

            assert_snapshot!(graph.to_dot());
            assert_eq!(
                topo(graph),
                vec![
                    ActionNode::sync_workspace(),
                    ActionNode::setup_environment(SetupEnvironmentNode {
                        owner_project_id: None,
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

            let index = builder.setup_environment_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(!output.report.nodes(index).is_empty());

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
                        owner_project_id: None,
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

            let index = builder.setup_environment_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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

            let index = builder.setup_environment_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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

            let index = builder.setup_environment_root(&spec);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

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

            builder.setup_environment_root(&spec);

            let graph = builder.build().await.unwrap().graph;

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }

    mod setup_toolchain {
        use super::*;

        // Both require `tc-tier3`, so a sibling must still link to it, even
        // though it was already setup through the other one
        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_requirements_path_local_across_siblings() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());
            let mut builder = container
                .create_builder(container.create_workspace_graph().await)
                .await;

            builder.setup_toolchain(&create_tier_spec_with_name("tc-tier3-reqs"), None);
            builder.setup_toolchain(&create_tier_spec_with_name("tc-tier2-reqs"), None);

            let graph = builder.build().await.unwrap().graph;

            assert!(
                graph
                    .dependencies_of(&ActionNode::setup_toolchain(SetupToolchainNode {
                        toolchain: create_tier_spec_with_name("tc-tier2-reqs"),
                    }))
                    .contains(&"SetupToolchain(tc-tier3:1.2.3)".to_owned())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_graph_if_tier1() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let ts = ToolchainSpec::new_global(Id::raw("tc-tier1"));

            builder.setup_toolchain(&ts, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&ts, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&system, None);
            builder.setup_toolchain(&node, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&node1, None);
            builder.setup_toolchain(&node2, None);
            builder.setup_toolchain(&node3, None);
            builder.setup_toolchain(&node4, None);
            builder.setup_toolchain(&node5, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&node, None);
            builder.setup_toolchain(&node, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&system, None);
            builder.setup_toolchain(&node, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&system, None);
            builder.setup_toolchain(&node, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&system, None);
            builder.setup_toolchain(&node, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&node, None);

            let graph = builder.build().await.unwrap().graph;

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

            builder.setup_toolchain(&node, None);

            let graph = builder.build().await.unwrap().graph;

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

            let index = builder.setup_toolchain(&node, None);

            let output = builder.build().await.unwrap();
            let graph = output.graph;

            assert!(output.report.nodes(index).is_empty());

            assert_snapshot!(graph.to_dot());
            assert_eq!(topo(graph), vec![]);
        }
    }

    mod sync_project {
        use super::*;

        // Both depend on `baz`, so a sibling must still link to it, even
        // though it was already synced through the other one
        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_dependencies_path_local_across_siblings() {
            let sandbox = create_sandbox("projects");

            sandbox.create_file("foo/moon.yml", "dependsOn: [bar, qux]\n");
            sandbox.create_file("bar/moon.yml", "dependsOn: [baz]\nlanguage: javascript\n");
            sandbox.create_file(
                "qux/moon.yml",
                "dependsOn: [baz]\nlanguage: rust\n\ntoolchains:\n  rust:\n    version: '1.90.0'\n",
            );

            let mut container = ActionGraphContainer::new(sandbox.path());
            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            builder.set_requirements(RunRequirements::default());
            builder.sync_project(wg.get_project("bar").unwrap());
            builder.sync_project(wg.get_project("qux").unwrap());

            let graph = builder.build().await.unwrap().graph;

            assert!(
                graph
                    .dependencies_of(&ActionNode::sync_project(SyncProjectNode {
                        project_id: Id::raw("qux"),
                    }))
                    .contains(&"SyncProject(baz)".to_owned())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn graphs_single() {
            let sandbox = create_sandbox("projects");
            let mut container = ActionGraphContainer::new(sandbox.path());

            let wg = container.create_workspace_graph().await;
            let mut builder = container.create_builder(wg.clone()).await;

            let bar = wg.get_project("bar").unwrap();
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(bar.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(foo.clone());

            let bar = wg.get_project("bar").unwrap();
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(bar.clone());

            let qux = wg.get_project("qux").unwrap();
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(qux.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(foo.clone());

            let qux = wg.get_project("qux").unwrap();
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(qux.clone());

            let graph = builder.build().await.unwrap().graph;

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

            builder.set_requirements(RunRequirements::default());
            builder.sync_project(foo.clone());
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(foo.clone());
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(foo.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(bar.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(bar.clone());

            let graph = builder.build().await.unwrap().graph;

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
            builder.set_requirements(RunRequirements::default());
            builder.sync_project(bar.clone());

            let graph = builder.build().await.unwrap().graph;

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

            builder.sync_workspace();

            let graph = builder.build().await.unwrap().graph;

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

            builder.sync_workspace();
            builder.sync_workspace();
            builder.sync_workspace();

            let graph = builder.build().await.unwrap().graph;

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

            builder.sync_workspace();

            let graph = builder.build().await.unwrap().graph;

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

                builder.set_requirements(RunRequirements::default());
                let request = builder.run_tasks_with_plan(&plan);
                let BuildOutput {
                    context,
                    graph,
                    mut report,
                } = builder.build().await.unwrap();
                let partition = report.take_partition(request).unwrap();

                assert_eq!(partition.targets.len(), 1);

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

                builder.set_requirements(RunRequirements::default());
                let request = builder.run_tasks_with_plan(&plan);
                let BuildOutput {
                    context,
                    graph,
                    mut report,
                } = builder.build().await.unwrap();
                let partition = report.take_partition(request).unwrap();

                assert_eq!(partition.targets.len(), 2);

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

                builder.set_requirements(RunRequirements::default());
                let request = builder.run_tasks_with_plan(&plan);
                let BuildOutput {
                    context,
                    graph,
                    mut report,
                } = builder.build().await.unwrap();
                let partition = report.take_partition(request).unwrap();

                assert_eq!(partition.targets.len(), 1);

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

                builder.set_requirements(RunRequirements {
                    job: Some(0),
                    job_total: Some(2),
                    ..Default::default()
                });
                let request = builder.run_tasks_with_plan(&plan);
                let BuildOutput {
                    context,
                    graph,
                    mut report,
                } = builder.build().await.unwrap();
                let partition = report.take_partition(request).unwrap();

                assert_eq!(partition.targets.len(), 1);
                assert_eq!(partition.size, Some(1));

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

                builder.set_requirements(RunRequirements {
                    job: Some(1),
                    job_total: Some(2),
                    ..Default::default()
                });
                let request = builder.run_tasks_with_plan(&plan);
                let BuildOutput {
                    context,
                    graph,
                    mut report,
                } = builder.build().await.unwrap();
                let partition = report.take_partition(request).unwrap();

                assert_eq!(partition.targets.len(), 1);
                assert_eq!(partition.size, Some(1));

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

                builder.set_requirements(RunRequirements::default());
                builder.run_tasks_with_plan(&plan);

                builder.build().await.unwrap();
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

                builder.set_requirements(RunRequirements {
                    job: Some(5),
                    job_total: Some(2),
                    ..Default::default()
                });
                builder.run_tasks_with_plan(&plan);

                builder.build().await.unwrap();
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

                builder.set_requirements(RunRequirements {
                    job: Some(0),
                    job_total: Some(5),
                    ..Default::default()
                });
                builder.run_tasks_with_plan(&plan);

                builder.build().await.unwrap();
            }
        }

        mod graph_options {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn applies_no_dependencies_from_plan() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    graph: GraphBlock {
                        no_dependencies: Some(true),
                        ..Default::default()
                    },
                    targets: TargetsBlock::Included(vec![
                        TargetLocator::parse("deps:chain3").unwrap(),
                    ]),
                    ..Default::default()
                };

                builder.set_requirements(RunRequirements {
                    no_dependencies: plan.graph.no_dependencies.unwrap_or_default(),
                    ..Default::default()
                });
                builder.run_tasks_with_plan(&plan);

                let graph = builder.build().await.unwrap().graph;

                assert_eq!(extract_run_task_targets(graph), ["deps:chain3"]);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn applies_dependents_from_plan() {
                let sandbox = create_sandbox("tasks");
                let mut container = ActionGraphContainer::new(sandbox.path());

                let mut builder = container
                    .create_builder(container.create_workspace_graph().await)
                    .await;

                let plan = ExecutionPlan {
                    graph: GraphBlock {
                        dependents: Some(Depth::DIRECT),
                        ..Default::default()
                    },
                    targets: TargetsBlock::Included(vec![
                        TargetLocator::parse("base:build").unwrap(),
                    ]),
                    ..Default::default()
                };

                builder.set_requirements(RunRequirements {
                    dependents: plan.graph.dependents.unwrap_or_default(),
                    ..Default::default()
                });
                builder.run_tasks_with_plan(&plan);

                let graph = builder.build().await.unwrap().graph;

                assert_snapshot!(graph.to_dot());
            }
        }
    }
}
