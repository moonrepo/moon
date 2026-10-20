use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::UnresolvedVersionSpec;
use moon_layout_graph::*;
use moon_project::Project;
use moon_test_utils::WorkspaceMocker;
use moon_toolchain::{DependenciesWorkspaceRole, ToolchainSpec};
use moon_workspace_graph::WorkspaceGraph;
use starbase_sandbox::{Sandbox, create_sandbox};
use std::sync::Arc;

fn tc() -> Id {
    Id::raw("tc-tier2")
}

fn path(value: &str) -> WorkspaceRelativePathBuf {
    WorkspaceRelativePathBuf::from(value)
}

fn spec(version: &str) -> ToolchainSpec {
    ToolchainSpec::new(tc(), UnresolvedVersionSpec::parse(version).unwrap())
}

fn ids(values: &[&str]) -> Vec<Id> {
    values.iter().map(|value| Id::raw(*value)).collect()
}

struct Container {
    app_context: Arc<moon_app_context::AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
}

impl Container {
    async fn new(sandbox: &Sandbox) -> Self {
        Self::new_with(sandbox, |mocker| mocker).await
    }

    async fn new_with(
        sandbox: &Sandbox,
        configure: impl FnOnce(WorkspaceMocker) -> WorkspaceMocker,
    ) -> Self {
        let mocker = configure(
            WorkspaceMocker::new(sandbox.path())
                .load_default_configs()
                .with_test_toolchains()
                .update_toolchains_config(|config| {
                    // Locate roots from each project, instead of the working dir
                    config
                        .plugins
                        .get_mut(&tc())
                        .unwrap()
                        .config
                        .insert("testLocateRoots".into(), serde_json::Value::Bool(true));
                }),
        )
        .with_global_envs();

        Self {
            app_context: Arc::new(mocker.mock_app_context()),
            workspace_graph: Arc::new(mocker.mock_workspace_graph().await),
        }
    }

    fn builder(&self) -> LayoutGraphBuilder {
        LayoutGraphBuilder::new(
            Arc::clone(&self.app_context),
            Arc::clone(&self.workspace_graph),
        )
    }

    fn project(&self, id: &str) -> Arc<Project> {
        self.workspace_graph.get_project(id).unwrap()
    }

    async fn place(&self, project_ids: &[&str]) -> LayoutGraph {
        let mut builder = self.builder();

        builder
            .place(project_ids.iter().map(|id| (self.project(id), tc())))
            .await
            .unwrap();

        builder.build()
    }
}

fn get_root<'graph>(graph: &'graph LayoutGraph, root: &str) -> &'graph ToolchainDependenciesRoot {
    graph.root(
        graph
            .find_root(&tc(), &path(root))
            .unwrap_or_else(|| panic!("Missing root {root:?}")),
    )
}

/// The fields of a root that must not depend on which members were placed.
fn root_fields(
    root: &ToolchainDependenciesRoot,
) -> (
    RootKind,
    Option<Id>,
    ToolchainSpec,
    serde_json::Value,
    RootFiles,
    RootCapabilities,
) {
    (
        root.kind.clone(),
        root.owner_project.clone(),
        root.spec.clone(),
        root.config.as_ref().clone(),
        root.files.clone(),
        root.capabilities,
    )
}

mod place {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn places_members_in_a_workspace_root() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let graph = container.place(&["a", "web"]).await;

        let a = graph.binding(&Id::raw("a"), &tc()).unwrap();
        let web = graph.binding(&Id::raw("web"), &tc()).unwrap();

        assert_eq!(a.role(), Some(DependenciesWorkspaceRole::WorkspaceMember));
        assert_eq!(web.role(), Some(DependenciesWorkspaceRole::WorkspaceMember));
        assert_eq!(a.root_index(), web.root_index());

        let root = get_root(&graph, "");

        assert_eq!(root.toolchain, tc());
        assert_eq!(
            root.kind,
            RootKind::Workspace {
                members: vec!["packages/*".into(), "apps/*".into(), "nested".into()]
            }
        );
        assert_eq!(root.owner_project, Some(Id::raw("root")));
        assert_eq!(root.spec, ToolchainSpec::new_global(tc()));
        assert_eq!(root.members, ids(&["a", "web"]));
        assert_eq!(
            root.files,
            RootFiles {
                lockfiles: vec![path("tc.lock")],
                manifests: vec![path("tc.root.cfg")],
                vendor_dir: Some(path("vendor")),
            }
        );
        assert_eq!(
            root.capabilities,
            RootCapabilities {
                install_dependencies: true,
                setup_environment: false,
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_the_root_project_config_for_a_workspace_root() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;

        // Also when only a member located it
        for project_id in ["root", "a"] {
            let graph = container.place(&[project_id]).await;
            let root = get_root(&graph, "");

            assert_eq!(root.owner_project, Some(Id::raw("root")));
            assert_eq!(root.config["rootKey"], serde_json::Value::Bool(true));
            assert_eq!(
                root.config["testLocateRoots"],
                serde_json::Value::Bool(true)
            );
        }

        let graph = container.place(&["root"]).await;

        assert_eq!(
            graph.binding(&Id::raw("root"), &tc()).unwrap().role(),
            Some(DependenciesWorkspaceRole::WorkspaceRoot)
        );
    }

    // The root's actions use the root project's version, while members that
    // don't override it run their tasks with the workspace's
    #[tokio::test(flavor = "multi_thread")]
    async fn uses_the_root_project_version_for_a_workspace_root() {
        let sandbox = create_sandbox("layout");

        sandbox.create_file(
            "moon.yml",
            "toolchains:\n  default: 'tc-tier2'\n  tc-tier2:\n    version: '4.0.0'",
        );

        let container = Container::new(&sandbox).await;
        let graph = container.place(&["a", "root"]).await;
        let root = get_root(&graph, "");

        assert_eq!(root.spec, spec("4.0.0"));
        assert_eq!(root.config["version"], "4.0.0");
        assert_eq!(
            graph.binding(&Id::raw("root"), &tc()).unwrap().spec,
            spec("4.0.0")
        );
        assert_eq!(
            graph.binding(&Id::raw("a"), &tc()).unwrap().spec,
            ToolchainSpec::new_global(tc())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_the_workspace_config_if_the_root_project_disables_it() {
        let sandbox = create_sandbox("layout");

        sandbox.create_file("moon.yml", "toolchains:\n  tc-tier2: false");

        let container = Container::new(&sandbox).await;
        let graph = container.place(&["a"]).await;
        let root = get_root(&graph, "");

        assert_eq!(root.owner_project, Some(Id::raw("root")));
        assert_eq!(root.spec, ToolchainSpec::new_global(tc()));
        assert!(root.config.get("rootKey").is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_the_workspace_version_when_a_member_overrides_it() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let graph = container.place(&["b"]).await;

        let binding = graph.binding(&Id::raw("b"), &tc()).unwrap();

        assert_eq!(binding.spec, spec("2.0.0"));
        assert_eq!(binding.config["version"], "2.0.0");
        assert_eq!(get_root(&graph, "").spec, ToolchainSpec::new_global(tc()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_the_owner_version_and_config_for_a_package_root() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let graph = container.place(&["standalone"]).await;

        let binding = graph.binding(&Id::raw("standalone"), &tc()).unwrap();
        let root = get_root(&graph, "standalone");

        assert_eq!(binding.role(), Some(DependenciesWorkspaceRole::PackageRoot));
        assert_eq!(root.kind, RootKind::Package);
        assert_eq!(root.owner_project, Some(Id::raw("standalone")));
        assert_eq!(root.spec, spec("3.0.0"));
        assert_eq!(root.config["customKey"], "standalone");
        assert_eq!(root.members, ids(&["standalone"]));
        assert_eq!(
            root.files,
            RootFiles {
                lockfiles: vec![path("standalone/tc.lock")],
                manifests: vec![],
                vendor_dir: None,
            }
        );

        // The enclosing workspace isn't involved
        assert!(graph.find_root(&tc(), &path("")).is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn places_a_nested_workspace_in_its_own_root() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let graph = container.place(&["nested", "lib", "a"]).await;

        let nested = graph.binding(&Id::raw("nested"), &tc()).unwrap();
        let lib = graph.binding(&Id::raw("lib"), &tc()).unwrap();
        let a = graph.binding(&Id::raw("a"), &tc()).unwrap();

        // Even though the outer workspace's members match `nested`
        assert_eq!(
            nested.role(),
            Some(DependenciesWorkspaceRole::WorkspaceRoot)
        );
        assert_eq!(lib.role(), Some(DependenciesWorkspaceRole::WorkspaceMember));
        assert_eq!(nested.root_index(), lib.root_index());
        assert_ne!(nested.root_index(), a.root_index());

        let root = get_root(&graph, "nested");

        assert_eq!(
            root.kind,
            RootKind::Workspace {
                members: vec!["lib".into()]
            }
        );
        assert_eq!(root.owner_project, Some(Id::raw("nested")));
        assert_eq!(root.members, ids(&["lib", "nested"]));
        assert_eq!(get_root(&graph, "").members, ids(&["a"]));
        assert_eq!(graph.roots_of(&tc()).count(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn places_a_non_member_outside_the_root() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let graph = container.place(&["tools"]).await;

        assert_eq!(
            graph.binding(&Id::raw("tools"), &tc()).unwrap().placement,
            Placement::Outside { root: path("") }
        );

        // No root is created for it
        assert_eq!(graph.roots_of(&tc()).count(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn places_nowhere_without_tier_2() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder
            .place([(container.project("plain"), Id::raw("tc-tier1"))])
            .await
            .unwrap();

        let graph = builder.build();
        let binding = graph
            .binding(&Id::raw("plain"), &Id::raw("tc-tier1"))
            .unwrap();

        assert_eq!(binding.placement, Placement::None);
        assert_eq!(binding.spec, ToolchainSpec::new_global(Id::raw("tc-tier1")));
        assert!(graph.roots().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_a_toolchain_disabled_by_the_project() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let graph = container.place(&["off"]).await;

        assert!(graph.bindings(&Id::raw("off")).is_empty());
        assert!(graph.roots().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_pairs_that_are_already_placed() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder
            .place([
                (container.project("a"), tc()),
                (container.project("a"), tc()),
            ])
            .await
            .unwrap();
        builder
            .place([(container.project("a"), tc())])
            .await
            .unwrap();

        let graph = builder.build();

        assert_eq!(graph.bindings(&Id::raw("a")).len(), 1);
        assert_eq!(get_root(&graph, "").members, ids(&["a"]));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn root_fields_dont_depend_on_which_members_are_placed() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;

        let by_member = container.place(&["b"]).await;
        let by_owner = container.place(&["root"]).await;
        let by_all = container.place(&["web", "root", "b", "a"]).await;

        let expected = root_fields(get_root(&by_all, ""));

        assert_eq!(root_fields(get_root(&by_member, "")), expected);
        assert_eq!(root_fields(get_root(&by_owner, "")), expected);
        assert_eq!(
            get_root(&by_all, "").members,
            ids(&["a", "b", "root", "web"])
        );
    }
}

mod place_project {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn places_toolchains_only_used_by_tasks() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder
            .place_project(container.project("tools"))
            .await
            .unwrap();

        let graph = builder.build();

        assert_eq!(
            graph.binding(&Id::raw("tools"), &tc()).unwrap().placement,
            Placement::Outside { root: path("") }
        );
        assert!(!graph.is_complete());
    }
}

mod place_all {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn places_every_project() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder.place_all().await.unwrap();

        let graph = builder.build();

        assert!(graph.is_complete());
        assert_eq!(
            graph
                .roots_of(&tc())
                .map(|root| root.root.as_str())
                .collect::<Vec<_>>(),
            vec!["", "nested", "standalone"]
        );
        assert_eq!(
            get_root(&graph, "").members,
            ids(&["a", "b", "root", "web"])
        );
        assert_eq!(get_root(&graph, "nested").members, ids(&["lib", "nested"]));
        assert_eq!(get_root(&graph, "standalone").members, ids(&["standalone"]));
        assert_eq!(
            graph.binding(&Id::raw("tools"), &tc()).unwrap().placement,
            Placement::Outside { root: path("") }
        );
        assert!(graph.binding(&Id::raw("off"), &tc()).is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn matches_a_lazy_build() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder.place_all().await.unwrap();

        let full = builder.build();
        let lazy = container.place(&["web", "lib", "standalone"]).await;

        for root in ["", "nested", "standalone"] {
            assert_eq!(
                root_fields(get_root(&lazy, root)),
                root_fields(get_root(&full, root)),
                "root {root:?}"
            );
        }

        for project_id in ["web", "lib", "standalone"] {
            let lazy_binding = lazy.binding(&Id::raw(project_id), &tc()).unwrap();
            let full_binding = full.binding(&Id::raw(project_id), &tc()).unwrap();

            assert_eq!(lazy_binding.spec, full_binding.spec);
            assert_eq!(lazy_binding.config, full_binding.config);
            assert_eq!(lazy_binding.role(), full_binding.role());
        }
    }
}

mod place_workspace {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn places_from_the_workspace_root() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder.place_workspace([tc()]).await.unwrap();

        let graph = builder.build();
        let binding = graph.workspace_binding(&tc()).unwrap();
        let root = get_root(&graph, "");

        assert_eq!(
            binding.role(),
            Some(DependenciesWorkspaceRole::WorkspaceRoot)
        );
        assert_eq!(binding.spec, ToolchainSpec::new_global(tc()));
        assert_eq!(root.owner_project, Some(Id::raw("root")));
        assert_eq!(root.config["rootKey"], serde_json::Value::Bool(true));

        // A workspace binding isn't a member
        assert!(root.members.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_unconfigured_toolchains() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder.place_workspace([Id::raw("unknown")]).await.unwrap();

        let graph = builder.build();

        assert!(graph.workspace_binding(&Id::raw("unknown")).is_none());
        assert!(graph.roots().is_empty());
    }
}

mod formats {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn formats_for_the_visualizer() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder
            .place([
                (container.project("a"), tc()),
                (container.project("tools"), tc()),
            ])
            .await
            .unwrap();
        builder.place_workspace([tc()]).await.unwrap();

        let graph = builder.build();
        let json: serde_json::Value =
            serde_json::from_str(&graph.to_visualizer_json().unwrap()).unwrap();

        assert_eq!(
            json["nodes"],
            serde_json::json!([
                { "id": 0, "label": "(workspace)", "type": "layout-workspace" },
                { "id": 1, "label": "tc-tier2 @ .", "type": "layout-root" },
                { "id": 2, "label": "a", "type": "layout-project" },
                { "id": 3, "label": "tools", "type": "layout-project" },
            ])
        );
        assert_eq!(
            json["edges"],
            serde_json::json!([
                { "id": "1 -> 0", "source": 1, "target": 0, "label": "workspace-root" },
                { "id": "2 -> 1", "source": 2, "target": 1, "label": "workspace-member" },
                { "id": "3 -> 1", "source": 3, "target": 1, "label": "outside" },
            ])
        );
    }
}

mod diagnostics {
    use super::*;

    // The other test toolchains are also detected for projects with a
    // `tc.lock`, and only locate the working dir
    fn get_diagnostics(graph: &LayoutGraph) -> Vec<LayoutDiagnostic> {
        graph
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.toolchain() == &tc())
            .cloned()
            .collect()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn diagnoses_a_full_build() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;
        let mut builder = container.builder();

        builder.place_all().await.unwrap();

        let graph = builder.build();
        let diagnostics = get_diagnostics(&graph);

        assert_eq!(
            diagnostics,
            vec![
                // Overrides the version, which the workspace doesn't configure
                LayoutDiagnostic::Divergent {
                    project: Id::raw("b"),
                    toolchain: tc(),
                    root: path(""),
                    spec: spec("2.0.0"),
                    root_spec: ToolchainSpec::new_global(tc()),
                },
                // Its own workspace, while the outer workspace's members match `nested`
                LayoutDiagnostic::Shadowed {
                    project: Id::raw("nested"),
                    toolchain: tc(),
                    root: path("nested"),
                    enclosing_root: path(""),
                },
                LayoutDiagnostic::Outside {
                    project: Id::raw("tools"),
                    toolchain: tc(),
                    root: path(""),
                },
            ]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.level())
                .collect::<Vec<_>>(),
            [
                DiagnosticLevel::Info,
                DiagnosticLevel::Warning,
                DiagnosticLevel::Info
            ]
        );
    }

    // A root is only known once a placed project located it
    #[tokio::test(flavor = "multi_thread")]
    async fn only_diagnoses_known_roots() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;

        assert!(get_diagnostics(&container.place(&["nested"]).await).is_empty());
        assert_eq!(
            get_diagnostics(&container.place(&["nested", "a"]).await),
            vec![LayoutDiagnostic::Shadowed {
                project: Id::raw("nested"),
                toolchain: tc(),
                root: path("nested"),
                enclosing_root: path(""),
            }]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_diagnose_package_roots_or_members() {
        let sandbox = create_sandbox("layout");
        let container = Container::new(&sandbox).await;

        // A package root with an override, a member, and a member of a nested workspace
        assert!(get_diagnostics(&container.place(&["standalone", "a", "lib"]).await).is_empty());
    }
}

mod batch {
    use super::*;

    // Load `tc-tier2` from the provided test plugin, and record its locate calls
    async fn create_container(sandbox: &Sandbox, plugin_name: &str) -> Container {
        Container::new_with(sandbox, |mocker| {
            mocker
                .with_test_toolchain_from("tc-tier2", plugin_name)
                .update_toolchains_config(|config| {
                    config
                        .plugins
                        .get_mut(&tc())
                        .unwrap()
                        .config
                        .insert("testRecordInputs".into(), serde_json::Value::Bool(true));
                })
        })
        .await
    }

    // The starting dirs of each recorded call of the function
    fn get_recorded_calls(sandbox: &Sandbox, func: &str) -> Vec<Vec<String>> {
        let Ok(entries) = std::fs::read_dir(
            sandbox
                .path()
                .join(".moon/cache/tcInputs/tc-tier2")
                .join(func),
        ) else {
            return vec![];
        };

        let mut calls = entries
            .map(|entry| {
                let input: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap())
                        .unwrap();

                let mut dirs: Vec<String> = match input.get("starting_dirs") {
                    Some(dirs) => serde_json::from_value(dirs.clone()).unwrap(),
                    None => vec![input["starting_dir"].as_str().unwrap().to_owned()],
                };

                dirs.sort();
                dirs
            })
            .collect::<Vec<Vec<String>>>();

        calls.sort();
        calls
    }

    async fn build_full(container: &Container) -> serde_json::Value {
        let mut builder = container.builder();

        builder.place_all().await.unwrap();
        builder.place_workspace([tc()]).await.unwrap();

        serde_json::to_value(builder.build()).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn matches_locating_one_root_per_call() {
        let single_sandbox = create_sandbox("layout");
        let single = create_container(&single_sandbox, "tc-tier2").await;
        let batch_sandbox = create_sandbox("layout");
        let batch = create_container(&batch_sandbox, "tc-tier2-batch").await;

        assert_eq!(build_full(&single).await, build_full(&batch).await);

        for project_ids in [&["standalone", "a", "tools"][..], &["lib", "nested", "off"]] {
            assert_eq!(
                serde_json::to_value(single.place(project_ids).await).unwrap(),
                serde_json::to_value(batch.place(project_ids).await).unwrap(),
                "{project_ids:?}"
            );
        }
    }

    async fn place_all(container: &Container) {
        let mut builder = container.builder();

        builder.place_all().await.unwrap();
    }

    // Projects that override the toolchain's config (`root`, `b`, and
    // `standalone`) are located separately, as the config is part of the input
    #[tokio::test(flavor = "multi_thread")]
    async fn locates_once_for_each_config() {
        let sandbox = create_sandbox("layout");
        let container = create_container(&sandbox, "tc-tier2-batch").await;

        place_all(&container).await;

        assert_eq!(
            get_recorded_calls(&sandbox, "locate_dependencies_roots"),
            [
                vec!["/workspace"],
                vec![
                    "/workspace/apps/web",
                    "/workspace/nested",
                    "/workspace/nested/lib",
                    "/workspace/packages/a",
                    "/workspace/tools",
                ],
                vec!["/workspace/packages/b"],
                vec!["/workspace/standalone"],
            ]
        );
        assert!(get_recorded_calls(&sandbox, "locate_dependencies_root").is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_once_for_each_project_without_batch_support() {
        let sandbox = create_sandbox("layout");
        let container = create_container(&sandbox, "tc-tier2").await;

        place_all(&container).await;

        assert_eq!(
            get_recorded_calls(&sandbox, "locate_dependencies_root").len(),
            8
        );
        assert!(get_recorded_calls(&sandbox, "locate_dependencies_roots").is_empty());
    }
}
