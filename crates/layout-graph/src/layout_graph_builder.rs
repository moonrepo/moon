use crate::layout_graph::*;
use futures::StreamExt;
use futures::stream::FuturesOrdered;
use indexmap::IndexSet;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::Id;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_pdk_api::LocateDependenciesRootInput;
use moon_project::Project;
use moon_toolchain::{
    DependenciesWorkspace, DependenciesWorkspaceRole, ToolchainSpec, get_toolchain_spec,
};
use moon_toolchain_plugin::ToolchainPlugin;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

/// A binding whose root was located by its plugin, before it's placed.
struct ResolvedBinding {
    project: Option<Arc<Project>>,
    toolchain: Arc<ToolchainPlugin>,
    spec: ToolchainSpec,
    config: Arc<JsonValue>,
    capabilities: RootCapabilities,
    located: Option<(DependenciesWorkspace, Option<DependenciesWorkspaceRole>)>,
}

/// Locate the root of a toolchain for a project (or the workspace root),
/// with the same input as the action graph, so that plugin calls are shared.
async fn resolve_binding(
    app_context: Arc<AppContext>,
    spec: ToolchainSpec,
    project: Option<Arc<Project>>,
) -> miette::Result<ResolvedBinding> {
    let registry = &app_context.toolchain_registry;
    let toolchain = registry.load(&spec.id).await?;
    let config = Arc::new(match &project {
        Some(project) => registry.create_merged_config(&toolchain.id, &project.config),
        None => registry.create_config(&toolchain.id),
    });
    let mut capabilities = RootCapabilities::default();
    let mut located = None;

    // Only tier 2 toolchains locate a root
    if toolchain.supports_tier_2().await {
        let target_root = match &project {
            Some(project) => &project.root,
            None => &app_context.workspace_root,
        };

        if let Some(deps_workspace) = toolchain
            .locate_dependencies_root(LocateDependenciesRootInput {
                context: registry.create_context(),
                starting_dir: toolchain.to_virtual_path(target_root),
                toolchain_config: config.as_ref().to_owned(),
            })
            .await?
        {
            let role = toolchain.in_dependencies_workspace(&deps_workspace, target_root)?;

            located = Some((deps_workspace, role));
        }

        capabilities = RootCapabilities {
            install_dependencies: toolchain.has_func("install_dependencies").await,
            setup_environment: toolchain.has_func("setup_environment").await,
        };
    }

    Ok(ResolvedBinding {
        project,
        toolchain,
        spec,
        config,
        capabilities,
        located,
    })
}

/// Probe the toolchain's registered file names in a root.
fn probe_root_files(
    toolchain: &ToolchainPlugin,
    abs_root: &Path,
    root: &WorkspaceRelativePathBuf,
) -> RootFiles {
    let probe = |names: &[String]| {
        names
            .iter()
            .filter(|name| abs_root.join(name).is_file())
            .map(|name| root.join(name))
            .collect::<Vec<_>>()
    };

    RootFiles {
        lockfiles: probe(&toolchain.metadata.lock_file_names),
        manifests: probe(&toolchain.metadata.manifest_file_names),
        vendor_dir: toolchain
            .metadata
            .vendor_dir_name
            .as_ref()
            .filter(|name| abs_root.join(name).is_dir())
            .map(|name| root.join(name)),
    }
}

/// Builds a layout graph, lazily from the (project, toolchain) pairs that are
/// placed (like the ones a run needs), or fully from every project.
pub struct LayoutGraphBuilder {
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    graph: LayoutGraph,

    /// Projects by their absolute root, to find the owner project of a root.
    owners: Option<FxHashMap<PathBuf, Id>>,
}

impl LayoutGraphBuilder {
    pub fn new(app_context: Arc<AppContext>, workspace_graph: Arc<WorkspaceGraph>) -> Self {
        Self {
            app_context,
            workspace_graph,
            graph: LayoutGraph::default(),
            owners: None,
        }
    }

    /// Place the toolchain of each pair for its project. Pairs that are already
    /// placed are skipped, as are toolchains that the project disabled, or that
    /// aren't configured. Roots are located concurrently.
    pub async fn place<I>(&mut self, pairs: I) -> miette::Result<()>
    where
        I: IntoIterator<Item = (Arc<Project>, Id)>,
    {
        let mut seen = FxHashSet::default();
        let mut futures = FuturesOrdered::new();

        for (project, toolchain_id) in pairs {
            if self.graph.binding(&project.id, &toolchain_id).is_some()
                || !seen.insert((project.id.clone(), toolchain_id.clone()))
            {
                continue;
            }

            let Some(spec) = get_toolchain_spec(
                &self.app_context.toolchains_config,
                &toolchain_id,
                Some(&project.config),
            ) else {
                continue;
            };

            if spec.is_system() {
                continue;
            }

            futures.push_back(tokio::spawn(resolve_binding(
                Arc::clone(&self.app_context),
                spec,
                Some(project),
            )));
        }

        self.insert_all(futures).await
    }

    /// Place every enabled toolchain of the project, and of its tasks.
    pub async fn place_project(&mut self, project: Arc<Project>) -> miette::Result<()> {
        let toolchain_ids = self.get_project_toolchains(&project)?;

        self.place(
            toolchain_ids
                .into_iter()
                .map(|toolchain_id| (Arc::clone(&project), toolchain_id)),
        )
        .await
    }

    /// Place every project in the workspace graph, which completes the
    /// members of every root.
    pub async fn place_all(&mut self) -> miette::Result<()> {
        let mut pairs = vec![];

        for project in self.workspace_graph.get_projects()? {
            for toolchain_id in self.get_project_toolchains(&project)? {
                pairs.push((Arc::clone(&project), toolchain_id));
            }
        }

        self.place(pairs).await?;
        self.graph.complete = true;

        Ok(())
    }

    /// Place toolchains from the workspace root, without a project (like
    /// `moon setup`). Toolchains that are already placed, or that aren't
    /// configured, are skipped.
    pub async fn place_workspace<I>(&mut self, toolchain_ids: I) -> miette::Result<()>
    where
        I: IntoIterator<Item = Id>,
    {
        let mut seen = FxHashSet::default();
        let mut futures = FuturesOrdered::new();

        for toolchain_id in toolchain_ids {
            if self.graph.workspace_binding(&toolchain_id).is_some()
                || !seen.insert(toolchain_id.clone())
            {
                continue;
            }

            let Some(spec) =
                get_toolchain_spec(&self.app_context.toolchains_config, &toolchain_id, None)
            else {
                continue;
            };

            if spec.is_system() {
                continue;
            }

            futures.push_back(tokio::spawn(resolve_binding(
                Arc::clone(&self.app_context),
                spec,
                None,
            )));
        }

        self.insert_all(futures).await
    }

    /// Freeze the graph.
    pub fn build(mut self) -> LayoutGraph {
        for root in &mut self.graph.roots {
            root.members.sort();
            root.members.dedup();
        }

        for bindings in self.graph.bindings.values_mut() {
            bindings.sort_by(|a, d| a.toolchain.cmp(&d.toolchain));
        }

        self.graph
            .workspace_bindings
            .sort_by(|a, d| a.toolchain.cmp(&d.toolchain));

        self.graph
    }

    fn get_project_toolchains(&self, project: &Project) -> miette::Result<IndexSet<Id>> {
        let mut toolchain_ids = project
            .get_enabled_toolchains()
            .into_iter()
            .cloned()
            .collect::<IndexSet<_>>();

        // Tasks may use toolchains that their project doesn't
        for task in self.workspace_graph.tasks.get_many(&project.task_targets)? {
            toolchain_ids.extend(
                project
                    .get_enabled_toolchains_for_task(&task)
                    .into_iter()
                    .cloned(),
            );
        }

        Ok(toolchain_ids)
    }

    /// Insert resolved bindings in the order they were requested, so that
    /// root indices are deterministic.
    async fn insert_all(
        &mut self,
        mut futures: FuturesOrdered<tokio::task::JoinHandle<miette::Result<ResolvedBinding>>>,
    ) -> miette::Result<()> {
        if !futures.is_empty() {
            debug!(
                count = futures.len(),
                "Placing toolchains into the layout graph"
            );
        }

        while let Some(result) = futures.next().await {
            let resolved = result.into_diagnostic()??;

            self.insert(resolved)?;
        }

        Ok(())
    }

    fn insert(&mut self, resolved: ResolvedBinding) -> miette::Result<()> {
        let placement = match &resolved.located {
            Some((deps_workspace, Some(role))) => {
                let root = deps_workspace
                    .root
                    .relative_to(&self.app_context.workspace_root)
                    .into_diagnostic()?;
                let index = self.get_or_insert_root(&resolved, deps_workspace, &root, role)?;

                if let Some(project) = &resolved.project {
                    self.graph.roots[index.index()]
                        .members
                        .push(project.id.clone());
                }

                Placement::Root { index, role: *role }
            }
            // A root outside of the workspace can't be represented, and it
            // wouldn't be used, as the project isn't a member of it
            Some((deps_workspace, None)) => match deps_workspace
                .root
                .relative_to(&self.app_context.workspace_root)
            {
                Ok(root) => Placement::Outside { root },
                Err(_) => Placement::None,
            },
            None => Placement::None,
        };

        let binding = Binding {
            toolchain: resolved.spec.id.clone(),
            spec: resolved.spec,
            config: resolved.config,
            placement,
        };

        match resolved.project {
            Some(project) => {
                self.graph
                    .bindings
                    .entry(project.id.clone())
                    .or_default()
                    .push(binding);
            }
            None => {
                self.graph.workspace_bindings.push(binding);
            }
        };

        Ok(())
    }

    fn get_or_insert_root(
        &mut self,
        resolved: &ResolvedBinding,
        deps_workspace: &DependenciesWorkspace,
        root: &WorkspaceRelativePathBuf,
        role: &DependenciesWorkspaceRole,
    ) -> miette::Result<RootIndex> {
        let toolchain_id = &resolved.spec.id;
        let key = (toolchain_id.to_owned(), root.to_owned());

        if let Some(index) = self.graph.root_keys.get(&key) {
            return Ok(*index);
        }

        // The project whose root is this root, which is the resolved project
        // when it's at the root, otherwise found by path
        let owner_project = match (&resolved.project, role) {
            (
                Some(project),
                DependenciesWorkspaceRole::WorkspaceRoot | DependenciesWorkspaceRole::PackageRoot,
            ) => Some(project.id.clone()),
            _ => self.find_owner_project(&deps_workspace.root),
        };

        let (spec, config) = match &deps_workspace.members {
            // A workspace's actions run with the workspace's version and
            // config, whatever its projects override
            Some(_) => self.get_workspace_spec_and_config(toolchain_id),
            // A package's run with its owner project's, as it's the only one
            None => match &owner_project {
                Some(owner_id) => match &resolved.project {
                    Some(project) if &project.id == owner_id => {
                        (resolved.spec.clone(), Arc::clone(&resolved.config))
                    }
                    _ => {
                        let owner = self.workspace_graph.get_project(owner_id)?;

                        match get_toolchain_spec(
                            &self.app_context.toolchains_config,
                            toolchain_id,
                            Some(&owner.config),
                        ) {
                            Some(spec) => (
                                spec,
                                Arc::new(
                                    self.app_context
                                        .toolchain_registry
                                        .create_merged_config(toolchain_id, &owner.config),
                                ),
                            ),
                            // Disabled by the owner
                            None => self.get_workspace_spec_and_config(toolchain_id),
                        }
                    }
                },
                None => self.get_workspace_spec_and_config(toolchain_id),
            },
        };

        let index = RootIndex(self.graph.roots.len() as u32);

        self.graph.roots.push(ToolchainDependenciesRoot {
            toolchain: toolchain_id.to_owned(),
            root: root.to_owned(),
            kind: match &deps_workspace.members {
                Some(members) => RootKind::Workspace {
                    members: members.to_owned(),
                },
                None => RootKind::Package,
            },
            owner_project,
            spec,
            config,
            files: probe_root_files(&resolved.toolchain, &deps_workspace.root, root),
            capabilities: resolved.capabilities,
            members: vec![],
        });

        self.graph.root_keys.insert(key, index);

        Ok(index)
    }

    fn get_workspace_spec_and_config(&self, toolchain_id: &Id) -> (ToolchainSpec, Arc<JsonValue>) {
        (
            get_toolchain_spec(&self.app_context.toolchains_config, toolchain_id, None)
                .unwrap_or_else(|| ToolchainSpec::new_global(toolchain_id.to_owned())),
            Arc::new(
                self.app_context
                    .toolchain_registry
                    .create_config(toolchain_id),
            ),
        )
    }

    fn find_owner_project(&mut self, abs_root: &Path) -> Option<Id> {
        let workspace_root = &self.app_context.workspace_root;
        let workspace_graph = &self.workspace_graph;

        self.owners
            .get_or_insert_with(|| {
                workspace_graph
                    .projects
                    .sources()
                    .into_iter()
                    .map(|(id, source)| (source.to_logical_path(workspace_root), id.to_owned()))
                    .collect()
            })
            .get(abs_root)
            .cloned()
    }
}
