use crate::layout_graph::*;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use indexmap::IndexSet;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::Id;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_pdk_api::LocateDependenciesRootsInput;
use moon_project::Project;
use moon_toolchain::{
    DependenciesWorkspace, DependenciesWorkspaceRole, ToolchainSpec, get_toolchain_spec,
};
use moon_toolchain_plugin::ToolchainPlugin;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value as JsonValue;
use starbase_utils::glob::GlobSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

/// A binding whose root was located by its plugin, before it's placed.
struct ResolvedBinding {
    project: Option<Arc<Project>>,
    toolchain: Arc<ToolchainPlugin>,
    spec: ToolchainSpec,
    config: Arc<JsonValue>,
    located: Option<(DependenciesWorkspace, Option<DependenciesWorkspaceRole>)>,
}

/// A binding to place, whose root is located first, if its toolchain is
/// tier 2.
struct PendingBinding {
    project: Option<Arc<Project>>,
    toolchain: Arc<ToolchainPlugin>,
    spec: ToolchainSpec,
    config: Arc<JsonValue>,
    locates: bool,
}

impl PendingBinding {
    /// The directory to locate the root from, which is the project's root,
    /// or the workspace root without a project.
    fn get_target_root<'path>(&'path self, workspace_root: &'path Path) -> &'path Path {
        match &self.project {
            Some(project) => &project.root,
            None => workspace_root,
        }
    }
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

/// Diagnose the placements of the graph's projects, in project order.
fn diagnose(
    graph: &LayoutGraph,
    sources: &FxHashMap<Id, WorkspaceRelativePathBuf>,
) -> Vec<LayoutDiagnostic> {
    let mut diagnostics = vec![];

    // The member globs of each workspace root, compiled once
    let globsets = graph
        .roots
        .iter()
        .map(|root| match &root.kind {
            RootKind::Workspace { members } => GlobSet::new_cached(members).ok(),
            RootKind::Package => None,
        })
        .collect::<Vec<_>>();

    for (project_id, bindings) in &graph.bindings {
        for binding in bindings {
            match &binding.placement {
                Placement::Outside { root } => {
                    diagnostics.push(LayoutDiagnostic::Outside {
                        project: project_id.clone(),
                        toolchain: binding.toolchain.clone(),
                        root: root.clone(),
                    });
                }
                Placement::Root { index, .. } => {
                    let root = graph.root(*index);

                    // Like `in_dependencies_workspace`, an enclosing root matches
                    // the project's path relative to the root with its globs
                    if let Some(source) = sources.get(project_id) {
                        for (other_index, other) in graph.roots.iter().enumerate() {
                            if other_index == index.index() || other.toolchain != binding.toolchain
                            {
                                continue;
                            }

                            let Some(globset) = &globsets[other_index] else {
                                continue;
                            };

                            if let Ok(path) = source.strip_prefix(&other.root)
                                && !matches!(path.as_str(), "" | ".")
                                && globset.matches(path.as_str())
                            {
                                diagnostics.push(LayoutDiagnostic::Shadowed {
                                    project: project_id.clone(),
                                    toolchain: binding.toolchain.clone(),
                                    root: root.root.clone(),
                                    enclosing_root: other.root.clone(),
                                });
                            }
                        }
                    }

                    if binding.spec != root.spec {
                        diagnostics.push(LayoutDiagnostic::Divergent {
                            project: project_id.clone(),
                            toolchain: binding.toolchain.clone(),
                            root: root.root.clone(),
                            spec: binding.spec.clone(),
                            root_spec: root.spec.clone(),
                        });
                    }
                }
                Placement::None => {}
            };
        }
    }

    diagnostics
}

/// Builds a layout graph, lazily from the (project, toolchain) pairs that are
/// placed (like the ones a run needs), or fully from every project.
pub struct LayoutGraphBuilder {
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    graph: LayoutGraph,

    /// Projects by their absolute root, to find the owner project of a root.
    owners: Option<FxHashMap<PathBuf, Id>>,

    /// Roots that were inserted, whose capabilities are read once inserting
    /// is done, so that it's once per root, instead of once per binding.
    pending_capabilities: Vec<(RootIndex, Arc<ToolchainPlugin>)>,

    /// The source of every placed project, for diagnostics.
    sources: FxHashMap<Id, WorkspaceRelativePathBuf>,
}

impl LayoutGraphBuilder {
    pub fn new(app_context: Arc<AppContext>, workspace_graph: Arc<WorkspaceGraph>) -> Self {
        Self {
            app_context,
            workspace_graph,
            graph: LayoutGraph::default(),
            owners: None,
            pending_capabilities: vec![],
            sources: FxHashMap::default(),
        }
    }

    /// Place the toolchain of each pair for its project. Pairs that are already
    /// placed are skipped, as are toolchains that the project disabled, or that
    /// aren't configured.
    pub async fn place<I>(&mut self, pairs: I) -> miette::Result<()>
    where
        I: IntoIterator<Item = (Arc<Project>, Id)>,
    {
        let mut seen = FxHashSet::default();
        let mut requests = vec![];

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

            requests.push((spec, Some(project)));
        }

        self.place_requests(requests).await
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
        let mut requests = vec![];

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

            requests.push((spec, None));
        }

        self.place_requests(requests).await
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

        self.graph.diagnostics = diagnose(&self.graph, &self.sources);
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

    /// Locate the roots of the requested bindings (with a project, or from
    /// the workspace root without one), and insert them in the order they
    /// were requested, so that root indices are deterministic.
    async fn place_requests(
        &mut self,
        requests: Vec<(ToolchainSpec, Option<Arc<Project>>)>,
    ) -> miette::Result<()> {
        if requests.is_empty() {
            return Ok(());
        }

        debug!(
            count = requests.len(),
            "Placing toolchains into the layout graph"
        );

        let pending = self.prepare(requests).await?;
        let located = self.locate(&pending).await?;

        for (binding, deps_workspace) in pending.into_iter().zip(located) {
            let located = match deps_workspace {
                Some(deps_workspace) => {
                    let role = binding.toolchain.in_dependencies_workspace(
                        &deps_workspace,
                        binding.get_target_root(&self.app_context.workspace_root),
                    )?;

                    Some((deps_workspace, role))
                }
                None => None,
            };

            self.insert(ResolvedBinding {
                project: binding.project,
                toolchain: binding.toolchain,
                spec: binding.spec,
                config: binding.config,
                located,
            })?;
        }

        for (index, toolchain) in std::mem::take(&mut self.pending_capabilities) {
            self.graph.roots[index.index()].capabilities = RootCapabilities {
                install_dependencies: toolchain.has_func("install_dependencies").await,
                setup_environment: toolchain.has_func("setup_environment").await,
            };
        }

        Ok(())
    }

    /// Load the toolchain of each request (once per toolchain), and create
    /// its config, with the same input as the action graph.
    async fn prepare(
        &self,
        requests: Vec<(ToolchainSpec, Option<Arc<Project>>)>,
    ) -> miette::Result<Vec<PendingBinding>> {
        let registry = &self.app_context.toolchain_registry;
        let mut toolchains = FxHashMap::default();

        for toolchain in registry
            .load_many(
                requests
                    .iter()
                    .map(|(spec, _)| &spec.id)
                    .collect::<IndexSet<_>>(),
            )
            .await?
        {
            // Only tier 2 toolchains locate a root
            let locates = toolchain.supports_tier_2().await;

            toolchains.insert(toolchain.id.clone(), (toolchain, locates));
        }

        let mut pending = Vec::with_capacity(requests.len());

        for (spec, project) in requests {
            let (toolchain, locates) = &toolchains[&spec.id];
            let config = Arc::new(match &project {
                Some(project) => registry.create_merged_config(&toolchain.id, &project.config),
                None => registry.create_config(&toolchain.id),
            });

            pending.push(PendingBinding {
                project,
                toolchain: Arc::clone(toolchain),
                spec,
                config,
                locates: *locates,
            });
        }

        Ok(pending)
    }

    /// Locate the root of each pending binding, in the same order, with a call
    /// for each toolchain and config, as most projects share their toolchain's
    /// config. A call locates many roots at once if the plugin supports it.
    /// Calls for different toolchains run concurrently.
    async fn locate(
        &self,
        pending: &[PendingBinding],
    ) -> miette::Result<Vec<Option<DependenciesWorkspace>>> {
        let mut groups = FxHashMap::<(&Id, String), Vec<usize>>::default();

        for (index, binding) in pending.iter().enumerate() {
            if binding.locates {
                groups
                    .entry((&binding.toolchain.id, binding.config.to_string()))
                    .or_default()
                    .push(index);
            }
        }

        let context = self.app_context.toolchain_registry.create_context();
        let workspace_root = &self.app_context.workspace_root;
        let mut futures = FuturesUnordered::new();

        for indices in groups.into_values() {
            let first = &pending[indices[0]];
            let toolchain = Arc::clone(&first.toolchain);
            let input = LocateDependenciesRootsInput {
                context: context.clone(),
                starting_dirs: indices
                    .iter()
                    .map(|index| {
                        toolchain.to_virtual_path(pending[*index].get_target_root(workspace_root))
                    })
                    .collect(),
                toolchain_config: first.config.as_ref().to_owned(),
            };

            futures.push(tokio::spawn(async move {
                let roots = toolchain.locate_dependencies_roots(input).await?;

                Ok::<_, miette::Report>((indices, roots))
            }));
        }

        let mut located = (0..pending.len()).map(|_| None).collect::<Vec<_>>();

        while let Some(result) = futures.next().await {
            let (indices, roots) = result.into_diagnostic()??;

            for (index, root) in indices.into_iter().zip(roots) {
                located[index] = root;
            }
        }

        Ok(located)
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
                self.sources
                    .entry(project.id.clone())
                    .or_insert_with(|| project.source.clone());

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

        // A root's actions run with its owner project's version and config,
        // as it's the project at the root, like a package, or the root project
        // of a workspace (whatever its members override), otherwise with the
        // workspace's
        let (spec, config) = match &owner_project {
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
            capabilities: RootCapabilities::default(),
            members: vec![],
        });

        self.graph.root_keys.insert(key, index);
        self.pending_capabilities
            .push((index, Arc::clone(&resolved.toolchain)));

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
