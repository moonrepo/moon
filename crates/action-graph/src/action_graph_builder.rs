use crate::action_graph::ActionGraph;
use crate::action_graph_error::ActionGraphError;
use daggy::{Dag, Walker};
use miette::IntoDiagnostic;
use moon_action::{
    ActionNode, InstallDependenciesNode, RunTaskNode, SetupEnvironmentNode, SetupToolchainNode,
    SyncProjectNode,
};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, Depth};
use moon_app_context::AppContext;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_common::{Id, color, is_ci};
use moon_config::{EnvMap, PipelineActionSwitch, TaskDependencyConfig, TaskDependencyType};
use moon_exec_plan::{ExecutionPlan, TargetsBlock};
use moon_pdk_api::{DefineRequirementsInput, LocateDependenciesRootInput};
use moon_project::{Project, ProjectError};
use moon_query::{Criteria, build_query};
use moon_task::{Target, TargetError, TargetLocator, TargetProjectScope, TargetTaskScope, Task};
use moon_toolchain::{DependenciesWorkspace, DependenciesWorkspaceRole, ToolchainSpec};
use moon_workspace_graph::projects::ProjectGraphError;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Debug;
use std::mem;
use std::sync::Arc;
use tracing::{debug, instrument, trace, warn};

macro_rules! insert_node_if_missing {
    ($builder:ident, $node:expr) => {{
        let node = $node;

        match $builder.get_index_from_node(&node) {
            Some(index) => index,
            None => $builder.insert_node(node),
        }
    }};
}

macro_rules! insert_node_or_exit {
    ($builder:ident, $node:expr) => {{
        let node = $node;

        match $builder.get_index_from_node(&node) {
            Some(index) => {
                return Ok(Some(index));
            }
            None => $builder.insert_node(node),
        }
    }};
}

#[derive(Clone, Debug)]
pub struct RunRequirements {
    pub ci: bool,                 // Are we in a CI environment
    pub ci_check: bool,           // Check the `runInCI` option
    pub no_dependencies: bool,    // Don't insert dependency tasks that weren't selected
    pub dependents: Depth,        // Add dependent tasks of the selected tasks
    pub interactive: bool,        // Entire pipeline is interactive
    pub job: Option<usize>,       // Current job index
    pub job_total: Option<usize>, // Total amount of jobs
    pub skip_affected: bool,      // Skip all affected checks
    pub skip_persistent: bool,    // Skip persistent tasks that aren't a dependency
}

impl Default for RunRequirements {
    fn default() -> Self {
        Self {
            ci: is_ci(),
            ci_check: false,
            no_dependencies: false,
            dependents: Depth::NONE,
            interactive: false,
            job: None,
            job_total: None,
            skip_affected: false,
            skip_persistent: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct RunPartition {
    pub targets: FxHashMap<NodeIndex, Target>,
    pub size: Option<usize>,
}

/// The graph edges produced by a task's dependencies.
#[derive(Debug, Default)]
pub struct RunTaskDependencies {
    /// Dependencies that run *before* the task, linked from the task to each
    /// dependency, using the type of the dependency as the edge weight.
    pub requirements: Vec<(NodeIndex, TaskDependencyType)>,

    /// Dependencies that run *after* the task. These are linked in reverse,
    /// from the dependency to the task, so they must be linked by the caller
    /// that owns the task's node index.
    pub cleanups: Vec<NodeIndex>,
}

/// Why a task is being inserted into the graph.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TaskOrigin {
    /// Selected by the target locators.
    #[default]
    Requested,
    /// Added as a dependent of a selected task.
    Dependent,
    /// Inserted because another task in the graph depends on it.
    Dependency,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RunTaskState {
    pub origin: TaskOrigin,
}

pub struct ActionGraphBuilderOptions {
    pub install_dependencies: PipelineActionSwitch,
    pub setup_environment: PipelineActionSwitch,
    pub setup_toolchains: PipelineActionSwitch,
    pub sync_projects: PipelineActionSwitch,
    pub sync_project_dependencies: bool,
    pub sync_workspace: bool,
}

impl Default for ActionGraphBuilderOptions {
    fn default() -> Self {
        Self::new(true)
    }
}

impl ActionGraphBuilderOptions {
    pub fn new(state: bool) -> Self {
        Self {
            install_dependencies: state.into(),
            setup_environment: state.into(),
            setup_toolchains: state.into(),
            sync_projects: state.into(),
            sync_project_dependencies: state,
            sync_workspace: state,
        }
    }
}

pub struct ActionGraphBuilder<'query> {
    all_query: Option<Criteria<'query>>,
    app_context: Arc<AppContext>,
    graph: Dag<ActionNode, TaskDependencyType>,
    nodes: FxHashMap<ActionNode, NodeIndex>,
    options: ActionGraphBuilderOptions,
    workspace_graph: Arc<WorkspaceGraph>,

    // Affected tracking
    affected: Option<AffectedTracker>,
    // How far affected state flows through relations (marks from dependencies)
    affected_depth: Depth,
    changed_files: Option<FxHashSet<WorkspaceRelativePathBuf>>,

    // Target tracking
    //
    // Dependencies (that aren't a cleanup) of tasks that were inserted without
    // their dependencies (`no_dependencies`). They're linked to the dependency nodes
    // that are in the graph when it's built, regardless of the insertion order,
    // and the ones that aren't are ignored.
    no_dependencies_tasks: FxHashMap<Target, Vec<TaskDependencyConfig>>,
    ignored_dependencies: FxHashMap<Target, FxHashSet<Target>>,
    passthrough_targets: FxHashSet<Target>,
    primary_targets: FxHashSet<Target>,

    // Serial ordering edges added by `try_link_edge`, and the order that they
    // were added in. Tracked so the serial subtree walk doesn't follow them as
    // if they were real dependency edges (both use `TaskDependencyType::Required`),
    // which would let it escape into unrelated subtrees when nodes are shared
    // across serial parents, and so that they can be removed when a dependency
    // that is linked afterwards contradicts them.
    serial_edges: FxHashMap<EdgeIndex, usize>,
    serial_edges_count: usize,

    // Nodes that run *after* another node, as a cleanup. Tracked so that they
    // can be located without having to scan every edge in the graph.
    cleanup_indices: FxHashSet<NodeIndex>,

    // Targets that are depended on to run after (cleanup) or alongside (wait)
    // another task, which must always run their command when they do.
    cleanup_targets: FxHashSet<Target>,
    wait_targets: FxHashSet<Target>,
}

impl<'query> ActionGraphBuilder<'query> {
    pub fn new(
        app_context: Arc<AppContext>,
        workspace_graph: Arc<WorkspaceGraph>,
        options: ActionGraphBuilderOptions,
    ) -> miette::Result<Self> {
        debug!("Building action graph");

        Ok(ActionGraphBuilder {
            affected: None,
            all_query: None,
            app_context,
            graph: Dag::new(),
            nodes: FxHashMap::default(),
            options,
            affected_depth: Depth::Deep,
            no_dependencies_tasks: FxHashMap::default(),
            ignored_dependencies: FxHashMap::default(),
            passthrough_targets: FxHashSet::default(),
            primary_targets: FxHashSet::default(),
            serial_edges: FxHashMap::default(),
            serial_edges_count: 0,
            cleanup_indices: FxHashSet::default(),
            cleanup_targets: FxHashSet::default(),
            wait_targets: FxHashSet::default(),
            changed_files: None,
            workspace_graph,
        })
    }

    pub fn build(mut self) -> (ActionContext, ActionGraph) {
        self.link_present_dependencies();

        let mut context = ActionContext {
            affected: self.affected.take().map(|affected| affected.build()),
            ..ActionContext::default()
        };

        if !self.passthrough_targets.is_empty() {
            for target in mem::take(&mut self.passthrough_targets) {
                context.set_target_state(target, TargetState::Passthrough);
            }
        }

        if !self.ignored_dependencies.is_empty() {
            context.ignored_dependencies = mem::take(&mut self.ignored_dependencies);
        }

        if !self.primary_targets.is_empty() {
            context.primary_targets = mem::take(&mut self.primary_targets);
        }

        context.cleanup_targets = mem::take(&mut self.cleanup_targets);
        context.wait_targets = mem::take(&mut self.wait_targets);

        if let Some(files) = self.changed_files.take() {
            context.changed_files = files.to_owned();
        }

        // Edges that are implied by a longer path (like a task's edge to its project's sync,
        // when one of its dependencies already depends on it) are intentionally not removed,
        // as they don't change the order that actions run in, and a transitive reduction is
        // costly for large graphs, while other consumers (like cleanup) read direct edges

        let mut nodes = FxHashMap::default();

        // TODO switch to map_owned
        // Node indexes are preserved by `map`, so the cleanup indexes
        // collected during building remain accurate
        let graph = self.graph.map(
            |ni, node| {
                nodes.insert(ni, node.clone());
                ni
            },
            |_, edge| edge.to_owned(),
        );

        (
            context,
            ActionGraph::new_with_cleanups(graph, nodes, mem::take(&mut self.cleanup_indices)),
        )
    }

    pub fn get_spec(&self, toolchain_id: &Id, project: Option<&Project>) -> Option<ToolchainSpec> {
        match project {
            Some(project) => self.get_project_spec(toolchain_id, project),
            None => self.get_workspace_spec(toolchain_id),
        }
    }

    pub fn get_project_spec(&self, toolchain_id: &Id, project: &Project) -> Option<ToolchainSpec> {
        if let Some(config) = project.config.toolchains.get_plugin_config(toolchain_id) {
            if !config.is_enabled() {
                return None;
            }

            if let Some(version) = config.get_version() {
                return Some(ToolchainSpec::new(
                    toolchain_id.to_owned(),
                    version.to_owned(),
                ));
            }
        }

        self.get_workspace_spec(toolchain_id)
    }

    pub fn get_workspace_spec(&self, toolchain_id: &Id) -> Option<ToolchainSpec> {
        if let Some(config) = self
            .app_context
            .toolchains_config
            .get_plugin_config(toolchain_id)
        {
            return Some(match &config.version {
                Some(version) => ToolchainSpec::new(toolchain_id.to_owned(), version.to_owned()),
                None => ToolchainSpec::new_global(toolchain_id.to_owned()),
            });
        }

        None
    }

    pub fn set_affected(&mut self) -> miette::Result<()> {
        if self.affected.is_none() {
            self.affected = Some(AffectedTracker::new(
                Arc::clone(&self.workspace_graph),
                self.changed_files
                    .as_ref()
                    .expect("Changed files are required for affected tracking.")
                    .to_owned(),
            ));
        }

        Ok(())
    }

    pub fn set_query(&mut self, input: &'query str) -> miette::Result<()> {
        self.all_query = Some(build_query(input)?);

        Ok(())
    }

    pub fn set_changed_files(
        &mut self,
        changed_files: FxHashSet<WorkspaceRelativePathBuf>,
    ) -> miette::Result<()> {
        self.changed_files = Some(changed_files);

        Ok(())
    }

    pub async fn track_affected(&mut self, depth: Depth, ci_check: bool) -> miette::Result<()> {
        // If affected state flows through relations, then we must load all
        // projects into the graph so that the edges are created!
        if !depth.is_none() {
            debug!("Force loading all projects and tasks to determine relationships");

            self.workspace_graph.get_projects()?;
            self.workspace_graph.get_tasks_with_internal()?;
        }

        self.affected_depth = depth;
        self.set_affected()?;

        if let Some(affected) = self.affected.as_mut() {
            affected.set_ci_check(ci_check);
            affected.set_depth(depth);

            // Projects must be tracked up front so that all marks exist
            // before tasks are inserted into the graph. Tracking lazily
            // during insertion makes the result dependent on insertion
            // order, as a project or task marked through another one's
            // relationship walk never runs its own checks and walks,
            // starving transitive dependents of marks.
            affected.track_projects().await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn internal_install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<Option<NodeIndex>> {
        if spec.is_system() {
            return Ok(None);
        }

        // If installing dependencies is disabled, we still need to ensure the
        // toolchain has been setup (and indirectly, the workspace synced), so
        // that its binaries are available to tasks
        if !self.options.install_dependencies.is_enabled(&spec.id)
            || self
                .app_context
                .toolchains_config
                .get_plugin_config(&spec.id)
                .is_some_and(|cfg| !cfg.install_dependencies)
        {
            return self.setup_toolchain(spec, project).await;
        }

        let sync_workspace_index = self.sync_workspace().await?;
        let setup_toolchain_index = self.setup_toolchain(spec, project).await?;
        let toolchain_registry = &self.app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support this action, so skip and fall through
        if !toolchain.supports_tier_2().await {
            return Ok(setup_toolchain_index);
        }

        let target_root = match project {
            Some(project) => &project.root,
            None => &self.app_context.workspace_root,
        };

        // Only insert this action if a root was located
        if let Some(deps_workspace) = self.locate_dependencies_root(spec, project).await?
            && let Some(deps_role) =
                toolchain.in_dependencies_workspace(&deps_workspace, target_root)?
        {
            // The action is scoped to the located root, so that every project
            // resolving to it collapses into the same action
            let root = deps_workspace
                .root
                .relative_to(&self.app_context.workspace_root)
                .into_diagnostic()?;

            // Unless there's no workspace, in which case the root is the only
            // package, and the project that owns it is associated, so that
            // project-level toolchain config is passed to the plugin. The
            // root may also not be owned by a project at all
            let project_id = match deps_role {
                DependenciesWorkspaceRole::PackageRoot => project.map(|project| project.id.clone()),
                _ => None,
            };

            let setup_env_index = self
                .internal_setup_environment(
                    spec,
                    &root,
                    project_id.as_ref().and(project),
                    FxHashSet::default(),
                )
                .await?;

            // Only create this action if the plugin supports it
            if toolchain.has_func("install_dependencies").await {
                let index = insert_node_if_missing!(
                    self,
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: deps_workspace.members,
                        project_id,
                        root,
                        toolchain_id: spec.id.clone(),
                    })
                );

                self.link_first_available_edge(
                    index,
                    vec![setup_env_index, setup_toolchain_index, sync_workspace_index],
                )?;

                return Ok(Some(index));
            }

            // Otherwise pass through to setup environment
            if let Some(setup_env_index) = setup_env_index {
                self.link_first_available_edge(
                    setup_env_index,
                    vec![setup_toolchain_index, sync_workspace_index],
                )?;

                return Ok(Some(setup_env_index));
            }
        }

        // Or fallback entirely to setup toolchain
        Ok(setup_toolchain_index)
    }

    #[instrument(skip(self))]
    pub async fn install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: &Project,
    ) -> miette::Result<Option<NodeIndex>> {
        self.internal_install_dependencies(spec, Some(project))
            .await
    }

    #[instrument(skip(self))]
    pub async fn install_dependencies_by_project(
        &mut self,
        project: &Project,
    ) -> miette::Result<Vec<Option<NodeIndex>>> {
        self.install_dependencies_by_toolchains(project, &project.toolchains)
            .await
    }

    #[instrument(skip(self))]
    pub async fn install_dependencies_by_toolchains(
        &mut self,
        project: &Project,
        toolchains: &[Id],
    ) -> miette::Result<Vec<Option<NodeIndex>>> {
        let mut indexes = vec![];

        for toolchain_id in toolchains {
            if let Some(spec) = self.get_project_spec(toolchain_id, project) {
                indexes.push(self.install_dependencies(&spec, project).await?);
            }
        }

        Ok(indexes)
    }

    #[instrument(skip(self))]
    pub async fn install_dependencies_root(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if spec.is_system()
            || !self.options.install_dependencies.is_enabled(&spec.id)
            || self
                .app_context
                .toolchains_config
                .get_plugin_config(&spec.id)
                .is_some_and(|cfg| !cfg.install_dependencies)
        {
            return Ok(None);
        }

        // Only insert actions if the dependencies root is the workspace root
        if self
            .locate_dependencies_root(spec, None)
            .await?
            .is_none_or(|deps_workspace| deps_workspace.root != self.app_context.workspace_root)
        {
            return Ok(None);
        }

        self.internal_install_dependencies(spec, None).await
    }

    async fn locate_dependencies_root(
        &self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<Option<DependenciesWorkspace>> {
        let toolchain_registry = &self.app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support locating a root, so return
        // an empty output instead of failing the function call
        if !toolchain.supports_tier_2().await {
            return Ok(None);
        }

        toolchain
            .locate_dependencies_root(match project {
                Some(project) => LocateDependenciesRootInput {
                    context: toolchain_registry.create_context(),
                    starting_dir: toolchain.to_virtual_path(&project.root),
                    toolchain_config: toolchain_registry
                        .create_merged_config(&toolchain.id, &project.config),
                },
                None => LocateDependenciesRootInput {
                    context: toolchain_registry.create_context(),
                    starting_dir: toolchain.to_virtual_path(&self.app_context.workspace_root),
                    toolchain_config: toolchain_registry.create_config(&toolchain.id),
                },
            })
            .await
    }

    /// Run the provided task as a primary target, and when requested, its
    /// dependents (up to the depth) as primary targets as well.
    #[instrument(skip(self))]
    pub async fn run_task(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Option<NodeIndex>> {
        let index = self
            .internal_run_primary_task(task, reqs, TaskOrigin::Requested)
            .await?;

        for dep_task in self
            .internal_resolve_dependents(&[task], reqs.dependents)
            .await?
        {
            self.internal_run_primary_task(&dep_task, reqs, TaskOrigin::Dependent)
                .await?;
        }

        Ok(index)
    }

    #[cfg(debug_assertions)]
    pub async fn run_task_with_config(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        config: &TaskDependencyConfig,
    ) -> miette::Result<Option<NodeIndex>> {
        Box::pin(self.internal_run_task(task, reqs, Some(config), &RunTaskState::default())).await
    }

    #[instrument(skip(self))]
    pub async fn run_task_by_target<T: AsRef<Target> + Debug>(
        &mut self,
        target: T,
        reqs: &RunRequirements,
    ) -> miette::Result<FxHashSet<NodeIndex>> {
        let tasks = self
            .internal_resolve_tasks_from_target(target.as_ref(), false)
            .await?;

        self.internal_run_primary_tasks(tasks, reqs).await
    }

    #[instrument(skip(self))]
    pub async fn run_task_by_target_locator<T: AsRef<TargetLocator> + Debug>(
        &mut self,
        locator: T,
        reqs: &RunRequirements,
    ) -> miette::Result<FxHashSet<NodeIndex>> {
        let tasks = self
            .internal_resolve_tasks_from_target_locator(locator.as_ref(), false)
            .await?;

        self.internal_run_primary_tasks(tasks, reqs).await
    }

    #[instrument(skip(self))]
    pub async fn run_tasks<I: IntoIterator<Item = T> + Debug, T: AsRef<TargetLocator> + Debug>(
        &mut self,
        locators: I,
        reqs: RunRequirements,
    ) -> miette::Result<RunPartition> {
        let mut tasks = vec![];
        let mut partition = RunPartition::default();

        for locator in locators {
            tasks.extend(
                self.internal_resolve_tasks_from_target_locator(locator.as_ref(), false)
                    .await?,
            );
        }

        // Expand the selection with the dependents of the selected tasks up
        // front, so that the candidates don't depend on which tasks have been
        // inserted (or skipped) by the time the dependents are expanded
        let mut candidates = self
            .internal_collect_candidates(tasks, reqs.dependents)
            .await?;

        // Determine affected status of each task up front, as marking
        // lazily during insertion makes the result dependent on target
        // order: a task marked through another task's relationship walk
        // never runs its own checks and walks, starving its transitive
        // dependents of marks and dropping them from the graph
        let affected_depth = self.affected_depth;

        if !reqs.skip_affected
            && let Some(affected) = &mut self.affected
        {
            // When affected state flows through relations, every task must be
            // tracked, not just the candidates. A task is only marked through a
            // relation when the task on the other side of it has been marked
            // itself, and that task is quite often not a candidate
            if affected_depth.is_none() {
                let tasks = candidates
                    .iter()
                    .map(|(task, _)| Arc::clone(task))
                    .collect::<Vec<_>>();

                affected.track_tasks_by_instance(&tasks).await?;
            } else {
                affected.track_tasks().await?;
            }
        }

        // Now partition the candidates based on the job information, which
        // includes the expanded dependents, so that they're spread across jobs
        if let Some(job_index) = reqs.job
            && let Some(job_total) = reqs.job_total
            && job_total > 0
        {
            if job_index > job_total - 1 {
                return Err(ActionGraphError::InvalidJobIndex {
                    index: job_index,
                    total: job_total,
                }
                .into());
            }

            // If we are going to parallelize, then we need to filter the
            // candidates based on affected state before partitioning!
            if !reqs.skip_affected {
                let mut new_candidates = vec![];

                for (task, origin) in candidates {
                    if self.is_task_affected(&task, &reqs)? {
                        new_candidates.push((task, origin));
                    }
                }

                candidates = new_candidates;
            }

            // Then slice and partition the candidates based on the job index and total
            let size = candidates.len().div_ceil(job_total);
            let (start, stop) =
                // beginning
                if job_index == 0 {
                    (0, size)
                }
                // end
                else if job_index == job_total - 1 {
                    ((size * job_index), candidates.len())
                }
                // middle
                else {
                    ((size * job_index), (size * (job_index + 1)))
                };

            if candidates.get(start).is_some() {
                if candidates.get(stop).is_some() {
                    candidates = candidates[start..stop].to_vec();
                } else {
                    candidates = candidates[start..].to_vec();
                }
            }

            partition.size = Some(size);
        }

        for (task, origin) in candidates {
            if let Some(index) = self.internal_run_primary_task(&task, &reqs, origin).await? {
                partition.targets.insert(index, task.target.clone());
            }
        }

        Ok(partition)
    }

    #[instrument(skip(self, plan))]
    pub async fn run_tasks_with_plan(
        &mut self,
        plan: &ExecutionPlan,
        mut reqs: RunRequirements,
    ) -> miette::Result<RunPartition> {
        match &plan.targets {
            TargetsBlock::Partitioned { jobs } => {
                let mut partition = RunPartition::default();

                if let Some(job_index) = reqs.job
                    && let Some(job_total) = reqs.job_total
                    && job_total > 0
                {
                    if job_index > job_total - 1 {
                        return Err(ActionGraphError::InvalidJobIndex {
                            index: job_index,
                            total: job_total,
                        }
                        .into());
                    }

                    if jobs.len() != job_total {
                        return Err(ActionGraphError::MismatchedPlanJobTotals {
                            plan_total: jobs.len(),
                            total: job_total,
                        }
                        .into());
                    }

                    // Reset job handling since we already did it
                    reqs.job = None;
                    reqs.job_total = None;

                    let targets = &jobs[job_index];

                    partition.targets = self.run_tasks(targets, reqs).await?.targets;
                    partition.size = Some(targets.len());
                } else {
                    return Err(ActionGraphError::InvalidPlanJobs.into());
                }

                Ok(partition)
            }
            TargetsBlock::Filtered { include } => self.run_tasks(include, reqs).await,
            TargetsBlock::Included(targets) => self.run_tasks(targets, reqs).await,
        }
    }

    #[instrument(skip(self))]
    pub async fn run_task_dependencies(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<RunTaskDependencies> {
        self.internal_run_task_dependencies(task, reqs, false).await
    }

    async fn internal_run_task_dependencies(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        cleanups_only: bool,
    ) -> miette::Result<RunTaskDependencies> {
        let parallel = task.options.run_deps_in_parallel;
        let mut deps = RunTaskDependencies::default();
        let mut previous_target_index: Option<NodeIndex> = None;
        let mut previous_standard_index: Option<NodeIndex> = None;

        for dep in &task.deps {
            if cleanups_only && !matches!(dep.type_of, TaskDependencyType::Cleanup) {
                continue;
            }

            for dep_task in self
                .internal_resolve_tasks_from_target(&dep.target, true)
                .await?
            {
                let dep_state = RunTaskState {
                    origin: TaskOrigin::Dependency,
                };

                if let Some(dep_index) =
                    Box::pin(self.internal_run_task(&dep_task, reqs, Some(dep), &dep_state)).await?
                {
                    // Cleanup dependencies run *after* the task, so the edge
                    // is reversed and must be linked by the caller, and they
                    // take no part in the serial chain
                    if matches!(dep.type_of, TaskDependencyType::Cleanup) {
                        self.cleanup_targets.insert(dep_task.target.clone());
                        deps.cleanups.push(dep_index);

                        continue;
                    }

                    if matches!(dep.type_of, TaskDependencyType::Wait) {
                        self.wait_targets.insert(dep_task.target.clone());
                    }

                    // When serial, this dependency's entire task subtree must
                    // run after the previous dependency — not just the
                    // dependency node itself. Otherwise its own transitive
                    // dependencies (grandchildren) would run in parallel with
                    // earlier serial dependencies. Cycle-forming edges are
                    // skipped, which can happen when the same task node appears
                    // in multiple serial dependency chains across parent tasks.
                    // Only dependencies that block the task are chained, as a
                    // `wait` dependency doesn't run to completion first.
                    let is_required = dep.type_of.is_required_type();

                    if !parallel && is_required {
                        self.link_serial_required_edges(
                            dep_index,
                            previous_target_index,
                            previous_standard_index,
                        );
                    }

                    // The parent always depends on each child directly, as
                    // serial chain edges alone can't guarantee this ordering
                    // when a chain edge is skipped for forming a cycle
                    deps.requirements.push((
                        dep_index,
                        // The optional type is only a marker within the task
                        // graph, the action graph has always used required
                        if is_required {
                            TaskDependencyType::Required
                        } else {
                            dep.type_of
                        },
                    ));

                    if is_required {
                        previous_target_index = Some(dep_index);

                        // A persistent dependency never completes, so it can't order
                        // the dependencies that come after it. Track the previous one
                        // that actually completes, so that they can be ordered against
                        // it instead
                        if !dep_task.is_persistent() {
                            previous_standard_index = Some(dep_index);
                        }
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Insert the provided task as a primary target (without expanding its dependents).
    async fn internal_run_primary_task(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        origin: TaskOrigin,
    ) -> miette::Result<Option<NodeIndex>> {
        let state = RunTaskState { origin };

        if let Some(index) = Box::pin(self.internal_run_task(task, reqs, None, &state)).await? {
            // Only track primary targets at the top-level run methods,
            // as these are explicitly called by pipeline consumers!
            self.primary_targets.insert(task.target.clone());

            return Ok(Some(index));
        }

        Ok(None)
    }

    async fn internal_run_primary_tasks(
        &mut self,
        tasks: Vec<Arc<Task>>,
        reqs: &RunRequirements,
    ) -> miette::Result<FxHashSet<NodeIndex>> {
        let mut indexes = FxHashSet::default();

        for (task, origin) in self
            .internal_collect_candidates(tasks, reqs.dependents)
            .await?
        {
            if let Some(index) = self.internal_run_primary_task(&task, reqs, origin).await? {
                indexes.insert(index);
            }
        }

        Ok(indexes)
    }

    /// Dedupe the selected tasks, and append their dependents (up to the depth).
    /// A task that was both selected and reached as a dependent is requested.
    async fn internal_collect_candidates(
        &mut self,
        tasks: Vec<Arc<Task>>,
        dependents: Depth,
    ) -> miette::Result<Vec<(Arc<Task>, TaskOrigin)>> {
        let mut seen = FxHashSet::default();
        let mut candidates = Vec::with_capacity(tasks.len());

        for task in tasks {
            if seen.insert(task.target.clone()) {
                candidates.push((task, TaskOrigin::Requested));
            }
        }

        if !dependents.is_none() {
            let seeds = candidates
                .iter()
                .map(|(task, _)| Arc::clone(task))
                .collect::<Vec<_>>();
            let seeds = seeds.iter().map(|task| task.as_ref()).collect::<Vec<_>>();

            for dep_task in self.internal_resolve_dependents(&seeds, dependents).await? {
                if seen.insert(dep_task.target.clone()) {
                    candidates.push((dep_task, TaskOrigin::Dependent));
                }
            }
        }

        Ok(candidates)
    }

    /// Resolve the dependents of the provided tasks, up to the depth, in
    /// breadth-first order. The provided tasks are excluded.
    ///
    /// When a query has been set, it bounds the expansion: dependents in
    /// projects that don't match it are not added, nor walked through.
    async fn internal_resolve_dependents(
        &mut self,
        tasks: &[&Task],
        depth: Depth,
    ) -> miette::Result<Vec<Arc<Task>>> {
        if depth.is_none() || tasks.is_empty() {
            return Ok(vec![]);
        }

        let query_project_ids = match &self.all_query {
            Some(all_query) => Some(
                self.workspace_graph
                    .query_projects(all_query)?
                    .into_iter()
                    .map(|project| project.id.clone())
                    .collect::<FxHashSet<_>>(),
            ),
            None => None,
        };

        let mut targets = vec![];

        // Cleanup dependencies are dependents in the task graph, as they run
        // after the task, but they're always linked through the task itself,
        // with their configured args and env (running them again as a dependent
        // would create a separate action that isn't ordered after the task),
        // so those edges are never followed
        self.workspace_graph.tasks.traverse_within_filtered_nodes(
            tasks,
            Direction::Incoming,
            depth,
            |edge| !matches!(edge, TaskDependencyType::Cleanup),
            |task| {
                query_project_ids.as_ref().is_none_or(|ids| {
                    task.target
                        .get_project_id()
                        .is_ok_and(|id| ids.contains(id))
                })
            },
            |task, _, _| {
                targets.push(task.target.clone());
            },
        );

        let mut dep_tasks = vec![];

        for target in targets {
            dep_tasks.extend(
                self.internal_resolve_tasks_from_target(&target, true)
                    .await?,
            );
        }

        Ok(dep_tasks)
    }

    #[instrument(skip(self))]
    async fn internal_resolve_tasks_from_target(
        &mut self,
        target: &Target,
        allow_internal: bool,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];

        // First, find all projects based on the target scope
        let (scope, scope_value) = target.get_project_scope();

        let (projects, bubble_error) = match scope {
            // :task
            TargetProjectScope::All => {
                let mut projects = vec![];

                if let Some(all_query) = &self.all_query {
                    projects.extend(self.workspace_graph.query_projects(all_query)?);
                } else {
                    projects.extend(self.workspace_graph.get_projects()?);
                };

                (projects, false)
            }
            // project:task
            TargetProjectScope::Id => {
                let project = self.workspace_graph.get_project(scope_value)?;

                (vec![project], true)
            }
            // #tag:task
            TargetProjectScope::Tag => {
                let projects = self
                    .workspace_graph
                    .query_projects(build_query(format!("projectTag={scope_value}").as_str())?)?;

                (projects, false)
            }
            // ^:task, ^build:task, etc.
            TargetProjectScope::Deps | TargetProjectScope::DepsOf(_) => {
                return Err(TargetError::NoDepsInRunContext.into());
            }
            // ~:task
            TargetProjectScope::OwnSelf => {
                return Err(TargetError::NoSelfInRunContext.into());
            }
        };

        // Second, find all tasks based on the task scope for each project
        let (scope, scope_value) = target.get_task_scope();

        for project in projects {
            let project_tasks = match scope {
                TargetTaskScope::Id => {
                    match self
                        .workspace_graph
                        .get_task_from_project(&project.id, scope_value)
                    {
                        Ok(task) => vec![task],
                        Err(error) => {
                            if bubble_error {
                                return Err(error);
                            } else {
                                continue;
                            }
                        }
                    }
                }
                TargetTaskScope::Tag => self
                    .workspace_graph
                    .get_tasks_from_project(&project.id)?
                    .into_iter()
                    .filter(|task| task.tags.iter().any(|tag| tag == scope_value))
                    .collect(),
            };

            for project_task in project_tasks {
                if !allow_internal && project_task.is_internal() {
                    if bubble_error {
                        return Err(ProjectError::UnknownTask {
                            task_id: project_task.id.to_string(),
                            project_id: project.id.to_string(),
                        }
                        .into());
                    } else {
                        continue;
                    }
                }

                tasks.push(project_task);
            }
        }

        Ok(tasks)
    }

    #[instrument(skip(self))]
    async fn internal_resolve_tasks_from_target_locator(
        &mut self,
        locator: &TargetLocator,
        allow_internal: bool,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];

        match locator {
            TargetLocator::GlobMatch {
                project,
                project_glob,
                task_glob,
                ..
            } => {
                let mut is_all = false;
                let mut do_query = false;
                let mut projects = vec![];

                // Query for all applicable projects first since we can't
                // query projects + tasks at the same time
                if let Some(glob) = project_glob {
                    let query = if let Some(tag_glob) = glob.strip_prefix('#') {
                        format!("projectTag~{tag_glob}")
                    } else {
                        format!("project~{glob}")
                    };

                    projects = self.workspace_graph.query_projects(build_query(&query)?)?;
                    do_query = !projects.is_empty();
                } else {
                    match project {
                        Some(TargetProjectScope::All) => {
                            is_all = true;
                            do_query = true;
                        }
                        _ => {
                            // Don't query for the other scopes,
                            // since they're not valid from the run context
                        }
                    };
                }

                // Then query for all tasks within the queried projects
                if do_query {
                    let mut query = if let Some(tag_glob) = task_glob.strip_prefix('#') {
                        format!("taskTag~{tag_glob}")
                    } else {
                        format!("task~{task_glob}")
                    };

                    if !is_all {
                        query = format!(
                            "project=[{}] && {query}",
                            projects
                                .into_iter()
                                .map(|project| project.id.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                    }

                    for task in self.workspace_graph.query_tasks(build_query(&query)?)? {
                        if !allow_internal && task.is_internal() {
                            continue;
                        }

                        tasks.push(task);
                    }
                }
            }
            TargetLocator::Qualified(target) => {
                let target = if target.project == TargetProjectScope::OwnSelf {
                    Target::new(
                        self.workspace_graph
                            .get_project_from_path(None)?
                            .id
                            .as_str(),
                        target.get_task_id()?,
                    )?
                } else {
                    target.to_owned()
                };

                tasks.extend(
                    self.internal_resolve_tasks_from_target(&target, allow_internal)
                        .await?,
                );
            }
            TargetLocator::DefaultProject(task_id) => {
                let project = self.workspace_graph.get_default_project().map_err(|_| {
                    ProjectGraphError::NoDefaultProjectForTask {
                        task_id: task_id.to_string(),
                    }
                })?;

                let target = Target::new(&project.id, task_id)?;

                tasks.extend(
                    self.internal_resolve_tasks_from_target(&target, allow_internal)
                        .await?,
                );
            }
        };

        Ok(tasks)
    }

    async fn internal_run_task(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        config: Option<&TaskDependencyConfig>,
        state: &RunTaskState,
    ) -> miette::Result<Option<NodeIndex>> {
        let project = self
            .workspace_graph
            .get_project(task.target.get_project_id()?)?;

        // Abort early if not affected
        if !self.is_task_affected(task, reqs)? {
            return Ok(None);
        }

        // These tasks shouldn't actually run, so filter them out
        if self.passthrough_targets.contains(&task.target) {
            debug!(
                task_target = task.target.as_str(),
                "Not running task {} because it has been marked as passthrough",
                color::id(&task.target.id),
            );

            return Ok(None);
        }

        // Only apply CI checks when requested
        if reqs.ci_check && !task.should_run(reqs.ci) {
            self.passthrough_targets.insert(task.target.clone());

            debug!(
                task_target = task.target.as_str(),
                "Not running task {} because {} has been configured not to",
                color::id(&task.target.id),
                color::property("runInCI"),
            );

            return Ok(None);
        }

        // Persistent tasks never complete, so they would keep the pipeline
        // running. When the targets were not requested (the command chose
        // them), they're only ran when another task depends on them. In CI,
        // they're also never ran as a dependent of another task
        if task.is_persistent()
            && state.origin != TaskOrigin::Dependency
            && (reqs.skip_persistent
                || (reqs.ci && reqs.ci_check && state.origin == TaskOrigin::Dependent))
        {
            debug!(
                task_target = task.target.as_str(),
                "Not running persistent task {} because it has not been requested, and no task depends on it",
                color::id(&task.target.id),
            );

            return Ok(None);
        }

        // Create the node
        let mut args = vec![];
        let mut env = EnvMap::default();

        if let Some(config) = config {
            args.extend(config.args.clone());
            env.extend(config.env.clone());
        }

        let node = ActionNode::run_task(RunTaskNode {
            args,
            env,
            interactive: task.is_interactive() || reqs.interactive,
            persistent: task.is_persistent(),
            priority: task.options.priority.get_level(),
            target: task.target.to_owned(),
            id: None,
        });

        // Check if the node exists to avoid all the overhead below
        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(index));
        }

        // Create initial edges
        let mut edges = vec![self.sync_project(&project, reqs).await?];

        edges.extend(
            self.install_dependencies_by_toolchains(&project, &task.toolchains)
                .await?,
        );

        // If no edges created, we should at minimum sync the workspace
        if edges.is_empty() || edges.iter().all(|edge| edge.is_none()) {
            edges.push(self.sync_workspace().await?);
        }

        // Insert and then link edges
        let index = self.insert_node(node);
        let mut deps = RunTaskDependencies::default();

        if !task.deps.is_empty() {
            let mut child_reqs = reqs.clone();
            child_reqs.skip_affected = true;

            if reqs.no_dependencies {
                // Dependencies that weren't selected are not inserted, but the
                // ones that are in the graph are still linked when it's built,
                // regardless of the order they're inserted in. Cleanup
                // dependencies must always run after the task, so they're
                // still inserted and linked here
                self.no_dependencies_tasks.insert(
                    task.target.clone(),
                    task.deps
                        .iter()
                        .filter(|dep| !matches!(dep.type_of, TaskDependencyType::Cleanup))
                        .cloned()
                        .collect(),
                );

                deps =
                    Box::pin(self.internal_run_task_dependencies(task, &child_reqs, true)).await?;
            } else {
                deps = Box::pin(self.run_task_dependencies(task, &child_reqs)).await?;
            }
        }

        self.link_available_edges(index, edges)?;
        self.link_task_dependencies(&[index], &deps)?;

        Ok(Some(index))
    }

    #[instrument(skip(self))]
    async fn internal_setup_environment(
        &mut self,
        spec: &ToolchainSpec,
        root: &WorkspaceRelativePathBuf,
        project: Option<&Project>,
        mut cycle: FxHashSet<&Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.setup_environment.is_enabled(&spec.id)
            || spec.is_system()
            || cycle.contains(&spec.id)
        {
            return Ok(None);
        }

        let toolchain_registry = &self.app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;
        let mut edges = vec![];
        let mut env_edges = vec![];

        cycle.insert(&spec.id);

        // Toolchain may depend on others
        let output = toolchain
            .define_requirements(DefineRequirementsInput {
                context: toolchain_registry.create_context(),
                toolchain_config: match project {
                    Some(project) => {
                        toolchain_registry.create_merged_config(&toolchain.id, &project.config)
                    }
                    None => toolchain_registry.create_config(&toolchain.id),
                },
            })
            .await?;

        for require_id in output.requires {
            let require_id = Id::new(require_id)?;

            // Skip if itself or already in cycle
            if require_id == spec.id || cycle.contains(&require_id) {
                continue;
            }

            let Some(require_spec) = self.get_spec(&require_id, project) else {
                if output.for_setup_environment {
                    return Err(ActionGraphError::MissingToolchainRequirement {
                        id: spec.id.to_string(),
                        dep_id: require_id.to_string(),
                    }
                    .into());
                }

                continue;
            };

            // Requires the toolchain to be setup, not the environment!
            if output.for_setup_environment {
                edges.push(Box::pin(self.setup_toolchain(&require_spec, project)).await?);
            }

            // Required toolchains (like Node.js for JavaScript) typically don't
            // locate a dependencies root, as this toolchain does, so their
            // environment must be setup in this toolchain's root, before its own
            if let Some(env_index) = Box::pin(self.internal_setup_environment(
                &require_spec,
                root,
                project,
                cycle.clone(),
            ))
            .await?
            {
                env_edges.push(env_index);
            }
        }

        // Toolchain does not support it
        if !toolchain.has_func("setup_environment").await
            && edges.is_empty()
            && env_edges.is_empty()
        {
            return Ok(None);
        }

        edges.push(self.sync_workspace().await?);
        edges.push(self.setup_toolchain(spec, project).await?);

        // The node may already exist, but the edges must still be linked, as the
        // version is not part of its identity, so another version of the toolchain
        // (overridden by a project that shares the root) must also be setup first
        let index = insert_node_if_missing!(
            self,
            ActionNode::setup_environment(SetupEnvironmentNode {
                project_id: project.map(|p| p.id.clone()),
                root: root.clone(),
                toolchain_id: spec.id.clone(),
            })
        );

        self.link_available_edges(index, edges)?;
        self.link_environment_edges(index, env_edges);

        Ok(Some(index))
    }

    #[instrument(skip(self))]
    pub async fn setup_environment(
        &mut self,
        spec: &ToolchainSpec,
        root: &WorkspaceRelativePathBuf,
        project: &Project,
    ) -> miette::Result<Option<NodeIndex>> {
        self.internal_setup_environment(spec, root, Some(project), FxHashSet::default())
            .await
    }

    /// Setup the environment in the workspace root, without an associated
    /// project. Unlike the project-based flow, this inserts no actions at
    /// all unless the located dependencies root is the workspace root.
    #[instrument(skip(self))]
    pub async fn setup_environment_root(
        &mut self,
        spec: &ToolchainSpec,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.setup_environment.is_enabled(&spec.id) || spec.is_system() {
            return Ok(None);
        }

        // Only insert actions if the dependencies root is the workspace root
        if self
            .locate_dependencies_root(spec, None)
            .await?
            .is_none_or(|deps_workspace| deps_workspace.root != self.app_context.workspace_root)
        {
            return Ok(None);
        }

        self.internal_setup_environment(
            spec,
            &WorkspaceRelativePathBuf::new(),
            None,
            FxHashSet::default(),
        )
        .await
    }

    #[instrument(skip(self))]
    pub async fn setup_proto(&mut self) -> miette::Result<Option<NodeIndex>> {
        let index = insert_node_or_exit!(
            self,
            ActionNode::setup_proto(self.app_context.toolchains_config.proto.version.clone())
        );

        Ok(Some(index))
    }

    #[instrument(skip(self))]
    pub async fn setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> miette::Result<Option<NodeIndex>> {
        Box::pin(self.internal_setup_toolchain(spec, project, FxHashSet::default())).await
    }

    async fn internal_setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
        mut cycle: FxHashSet<&Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.setup_toolchains.is_enabled(&spec.id)
            || spec.is_system()
            || cycle.contains(&spec.id)
        {
            return Ok(None);
        }

        let toolchain_registry = &self.app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;
        let mut edges = vec![];

        cycle.insert(&spec.id);

        // Toolchain may depend on others
        let output = toolchain
            .define_requirements(DefineRequirementsInput {
                context: toolchain_registry.create_context(),
                toolchain_config: match project {
                    Some(project) => {
                        toolchain_registry.create_merged_config(&toolchain.id, &project.config)
                    }
                    None => toolchain_registry.create_config(&toolchain.id),
                },
            })
            .await?;

        if !output.requires.is_empty() && output.for_setup_toolchain {
            for require_id in output.requires {
                let require_id = Id::new(require_id)?;

                if require_id != spec.id {
                    // Skip if already in cycle
                    if cycle.contains(&require_id) {
                        continue;
                    }

                    if let Some(require_spec) = self.get_spec(&require_id, project) {
                        edges.push(
                            Box::pin(self.internal_setup_toolchain(
                                &require_spec,
                                project,
                                cycle.clone(),
                            ))
                            .await?,
                        );
                    } else {
                        return Err(ActionGraphError::MissingToolchainRequirement {
                            id: spec.id.to_string(),
                            dep_id: require_id.to_string(),
                        }
                        .into());
                    }
                }
            }
        }

        // Toolchain does not support tier 3 and does not require other toolchains
        if !toolchain.supports_tier_3().await && edges.is_empty() {
            return Ok(None);
        }

        edges.push(self.sync_workspace().await?);

        if spec.req.is_some() || self.app_context.toolchains_config.requires_proto() {
            edges.push(self.setup_proto().await?);
        }

        let index = insert_node_if_missing!(
            self,
            ActionNode::setup_toolchain(SetupToolchainNode {
                toolchain: spec.to_owned(),
            })
        );

        self.link_available_edges(index, edges)?;

        Ok(Some(index))
    }

    #[instrument(skip(self))]
    pub async fn sync_project(
        &mut self,
        project: &Project,
        reqs: &RunRequirements,
    ) -> miette::Result<Option<NodeIndex>> {
        Box::pin(self.internal_sync_project(project, reqs, FxHashSet::default())).await
    }

    async fn internal_sync_project(
        &mut self,
        project: &Project,
        reqs: &RunRequirements,
        mut cycle: FxHashSet<&Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.sync_projects.is_enabled(&project.id) || cycle.contains(&project.id) {
            return Ok(None);
        }

        // Return early if not affected
        if !self.is_project_affected(project, reqs)? {
            return Ok(None);
        }

        // Insert the node and edges
        let mut edges = vec![];

        cycle.insert(&project.id);

        if let Some(sync_workspace_index) = self.sync_workspace().await? {
            edges.push(sync_workspace_index);
        }

        let index = insert_node_or_exit!(
            self,
            ActionNode::sync_project(SyncProjectNode {
                project_id: project.id.clone(),
            })
        );

        // We should also depend on other projects
        if self.options.sync_project_dependencies {
            for dep_project_id in self.workspace_graph.projects.dependencies_of(project) {
                if cycle.contains(&dep_project_id) {
                    continue;
                }

                let dep_project = self.workspace_graph.get_project(&dep_project_id)?;

                if let Some(dep_project_index) =
                    Box::pin(self.internal_sync_project(&dep_project, reqs, cycle.clone())).await?
                    && index != dep_project_index
                {
                    edges.push(dep_project_index);
                }
            }
        }

        if !edges.is_empty() {
            self.link_required_edges(index, edges)?;
        }

        Ok(Some(index))
    }

    #[instrument(skip(self))]
    pub async fn sync_workspace(&mut self) -> miette::Result<Option<NodeIndex>> {
        if !self.options.sync_workspace {
            return Ok(None);
        }

        let index = insert_node_or_exit!(self, ActionNode::sync_workspace());

        Ok(Some(index))
    }

    // PRIVATE

    /// Link the dependencies of tasks that were inserted without them (`no_dependencies`)
    /// to the dependency nodes that are in the graph, as the ordering between
    /// tasks that do run must still be honored, regardless of the order that
    /// they were inserted in. When a dependency has multiple nodes (different
    /// args or env), the ones that match the dependency's config are preferred,
    /// otherwise all of them are linked. Dependencies without a node in the
    /// graph are ignored, so that the task doesn't wait on (or hash) them.
    ///
    /// Serial ordering (`runDepsInParallel: false`) is only applied when the
    /// dependencies are inserted, and is not applied here.
    fn link_present_dependencies(&mut self) {
        if self.no_dependencies_tasks.is_empty() {
            return;
        }

        let mut target_indices = FxHashMap::<Target, Vec<NodeIndex>>::default();

        for (node, index) in &self.nodes {
            if let ActionNode::RunTask(inner) = node {
                target_indices
                    .entry(inner.target.clone())
                    .or_default()
                    .push(*index);
            }
        }

        for indices in target_indices.values_mut() {
            indices.sort();
        }

        let mut no_dependencies_tasks = mem::take(&mut self.no_dependencies_tasks)
            .into_iter()
            .collect::<Vec<_>>();

        // Link in a deterministic order, so that the graph's edge order is stable
        no_dependencies_tasks.sort_by(|a, b| a.0.cmp(&b.0));

        for (target, deps) in no_dependencies_tasks {
            let Some(owner_indices) = target_indices.get(&target) else {
                continue;
            };

            for dep in deps {
                let Some(dep_indices) = target_indices.get(&dep.target) else {
                    self.ignored_dependencies
                        .entry(target.clone())
                        .or_default()
                        .insert(dep.target.clone());

                    continue;
                };

                // The optional type is only a marker within the task
                // graph, the action graph has always used required
                let edge_type = if dep.type_of.is_required_type() {
                    TaskDependencyType::Required
                } else {
                    dep.type_of
                };

                let matching_indices = dep_indices
                    .iter()
                    .copied()
                    .filter(|index| {
                        matches!(
                            self.graph.node_weight(*index),
                            Some(ActionNode::RunTask(inner))
                                if inner.args == dep.args && inner.env == dep.env
                        )
                    })
                    .collect::<Vec<_>>();

                let dep_indices = if matching_indices.is_empty() {
                    dep_indices.clone()
                } else {
                    matching_indices
                };

                if matches!(edge_type, TaskDependencyType::Wait) {
                    self.wait_targets.insert(dep.target.clone());
                }

                for owner_index in owner_indices {
                    if dep_indices
                        .iter()
                        .any(|dep_index| self.has_edge_of_type(*owner_index, *dep_index, edge_type))
                    {
                        continue;
                    }

                    for dep_index in &dep_indices {
                        if owner_index == dep_index {
                            continue;
                        }

                        // The task graph is acyclic, and conflicting serial
                        // ordering edges are removed, so this is unexpected
                        if let Err(error) =
                            self.link_edges(*owner_index, vec![(*dep_index, edge_type)])
                        {
                            warn!(
                                task_target = target.as_str(),
                                dependency_target = dep.target.as_str(),
                                "Unable to link dependency: {error}"
                            );
                        }
                    }
                }
            }
        }
    }

    fn get_index_from_node(&self, node: &ActionNode) -> Option<NodeIndex> {
        self.nodes.get(node).cloned()
    }

    fn link_first_available_edge(
        &mut self,
        index: NodeIndex,
        edges: Vec<Option<NodeIndex>>,
    ) -> miette::Result<()> {
        if let Some(edge) = edges.into_iter().flatten().next() {
            self.link_required_edges(index, vec![edge])?;
        }

        Ok(())
    }

    fn link_available_edges(
        &mut self,
        index: NodeIndex,
        edges: Vec<Option<NodeIndex>>,
    ) -> miette::Result<()> {
        self.link_required_edges(index, edges.into_iter().flatten().collect())
    }

    fn link_required_edges(
        &mut self,
        index: NodeIndex,
        edges: Vec<NodeIndex>,
    ) -> miette::Result<()> {
        self.link_edges(
            index,
            edges
                .into_iter()
                .map(|edge| (edge, TaskDependencyType::Required))
                .collect(),
        )
    }

    fn link_edges(
        &mut self,
        index: NodeIndex,
        edges: Vec<(NodeIndex, TaskDependencyType)>,
    ) -> miette::Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        let mut added_edges = vec![];

        for (edge, edge_type) in edges {
            // A serial ordering edge may already link them in this direction,
            // which is a dependency from here on, and no longer a preference
            if let Some(edge_index) = self.find_edge_of_type(index, edge, edge_type) {
                self.serial_edges.remove(&edge_index);

                continue;
            }

            self.link_edge(index, edge, edge_type)?;

            added_edges.push(edge);
        }

        if !added_edges.is_empty() {
            trace!(
                index = index.index(),
                requires = ?added_edges.iter().map(|edge| edge.index()).collect::<Vec<_>>(),
                "Linking requirements for index"
            );
        }

        Ok(())
    }

    /// Link an edge between two nodes, for a dependency of any type. The nodes
    /// may already be connected in the other direction, which is a cycle,
    /// unless it's only because of serial ordering edges. Those are a
    /// preference, while this edge is a requirement, so the ones that
    /// contradict it are removed.
    fn link_edge(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        edge_type: TaskDependencyType,
    ) -> miette::Result<()> {
        if self.graph.add_edge(from, to, edge_type).is_err()
            && (!self.unlink_serial_edges(to, from)
                || self.graph.add_edge(from, to, edge_type).is_err())
        {
            return Err(ActionGraphError::WouldCycle {
                source_action: self.graph.node_weight(from).unwrap().label(),
                target_action: self.graph.node_weight(to).unwrap().label(),
                cycle: None,
            }
            .into());
        }

        Ok(())
    }

    /// Link the dependencies of a task to each of the nodes that run it.
    fn link_task_dependencies(
        &mut self,
        indices: &[NodeIndex],
        deps: &RunTaskDependencies,
    ) -> miette::Result<()> {
        for index in indices {
            self.link_edges(*index, deps.requirements.clone())?;
            self.link_cleanup_edges(*index, deps.cleanups.clone())?;
        }

        Ok(())
    }

    /// Link every cleanup dependency of the node at `index`. Cleanup actions
    /// run *after* the node they clean up, so the edge points from the cleanup
    /// action to the node, and not the other way around like a requirement.
    fn link_cleanup_edges(
        &mut self,
        index: NodeIndex,
        edges: Vec<NodeIndex>,
    ) -> miette::Result<()> {
        for edge in edges {
            // The cleanup action may also depend on the node in some other way,
            // which is a separate edge, as both relationships must be honored
            if !self.has_edge_of_type(edge, index, TaskDependencyType::Cleanup) {
                self.link_edge(edge, index, TaskDependencyType::Cleanup)?;

                trace!(
                    index = edge.index(),
                    cleans_up = ?[index.index()],
                    "Linking cleanup for index"
                );
            }

            self.cleanup_indices.insert(edge);
        }

        Ok(())
    }

    /// Link the environments of the toolchains that the environment at `index`
    /// requires. Toolchains may require each other (like Python and uv), so when
    /// the required environment already depends on this one, which happens when
    /// it was reached first, the edge would introduce a cycle, and is skipped.
    fn link_environment_edges(&mut self, index: NodeIndex, edges: Vec<NodeIndex>) {
        for edge in edges {
            if self.has_edge_of_type(index, edge, TaskDependencyType::Required) {
                continue;
            }

            if self
                .graph
                .add_edge(index, edge, TaskDependencyType::Required)
                .is_ok()
            {
                trace!(
                    index = index.index(),
                    requires = ?[edge.index()],
                    "Linking requirements for index"
                );
            } else {
                trace!(
                    index = index.index(),
                    requires = ?[edge.index()],
                    "Not linking required environment for index, as it would introduce a cycle"
                );
            }
        }
    }

    /// Add serial ordering edges from every task within `index`'s dependency
    /// subtree (including `index` itself) to the previous serial dependency, so
    /// that the whole subtree runs after it. The subtree is discovered by
    /// walking dependency edges — from a node to the tasks it requires — at link
    /// time (rather than collected during insertion) because a subtree shared
    /// with another target is inserted once and then reused via node
    /// deduplication. Only `RunTask` nodes are ordered; non-task nodes such as
    /// project syncs must not be forced to wait on the previous dependency.
    ///
    /// Non-persistent tasks are ordered against `previous_standard` — the
    /// previous dependency that actually completes — instead of `previous`,
    /// as a persistent dependency never completes and so cannot order them.
    ///
    /// Serial ordering edges (tracked in `serial_edges`) are skipped during the
    /// walk. They share the `Required` edge type with real dependencies, so
    /// following them — e.g. a `b -> a` edge left by an earlier serial parent on
    /// a shared node `b` — would let the walk escape `index`'s real subtree and
    /// wrongly order unrelated tasks. Cycle-forming edges are skipped when
    /// linked via [`Self::try_link_edge`].
    fn link_serial_required_edges(
        &mut self,
        index: NodeIndex,
        previous: Option<NodeIndex>,
        previous_standard: Option<NodeIndex>,
    ) {
        if previous.is_none() && previous_standard.is_none() {
            return;
        }

        let mut visited = FxHashSet::default();
        let mut ordered = vec![];
        let mut stack = vec![index];

        // Collect the subtree first, then link. Linking mutates the graph (it
        // adds `node -> previous` edges), so it must not run mid-walk or the new
        // edges would pollute the traversal. `ordered` keeps the link order
        // deterministic — and thus the graph's edge order stable — regardless
        // of set iteration order.
        while let Some(node_index) = stack.pop() {
            if !visited.insert(node_index) {
                continue;
            }

            ordered.push(node_index);

            let mut children = self.graph.children(node_index);

            while let Some((edge_index, child_index)) = children.walk_next(&self.graph) {
                if !self.serial_edges.contains_key(&edge_index)
                    // Only follow edges that block the node from running. A
                    // `cleanup` edge points from a cleanup action back to the
                    // node it cleans up, so following it would escape into the
                    // task that owns the cleanup and serialize it wrongly,
                    // while a `wait` edge doesn't run to completion first
                    && self
                        .graph
                        .edge_weight(edge_index)
                        .is_some_and(|weight| weight.is_required_type())
                    && matches!(
                        self.graph.node_weight(child_index),
                        Some(ActionNode::RunTask(_))
                    )
                {
                    stack.push(child_index);
                }
            }
        }

        for node_index in ordered {
            // A persistent action is marked as completed the moment it is
            // dispatched, as it never completes on its own, so ordering a
            // non-persistent action after one would simply run it immediately,
            // instead of after the dependency has actually finished. Order
            // those against the previous dependency that does complete
            let anchor = if self.is_persistent_index(node_index) {
                previous
            } else {
                previous_standard
            };

            if let Some(anchor) = anchor
                && anchor != node_index
            {
                self.try_link_edge(node_index, anchor);
            }
        }
    }

    /// Whether an edge of the provided type already links the two nodes. Edges
    /// of different types may link the same nodes (a task that both requires
    /// and cleans up after another), and the pipeline honors all of them.
    fn has_edge_of_type(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        edge_type: TaskDependencyType,
    ) -> bool {
        self.find_edge_of_type(from, to, edge_type).is_some()
    }

    fn find_edge_of_type(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        edge_type: TaskDependencyType,
    ) -> Option<EdgeIndex> {
        self.graph
            .graph()
            .edges_connecting(from, to)
            .find(|edge| *edge.weight() == edge_type)
            .map(|edge| edge.id())
    }

    /// Find the edges of a path that connects the two nodes, if there is one.
    fn find_path(&self, from: NodeIndex, to: NodeIndex) -> Option<Vec<EdgeIndex>> {
        let graph = self.graph.graph();
        let mut parents = FxHashMap::<NodeIndex, (NodeIndex, EdgeIndex)>::default();
        let mut visited = FxHashSet::from_iter([from]);
        let mut stack = vec![from];

        while let Some(node_index) = stack.pop() {
            if node_index == to {
                let mut path = vec![];
                let mut current = to;

                while let Some((parent_index, edge_index)) = parents.get(&current) {
                    path.push(*edge_index);
                    current = *parent_index;
                }

                return Some(path);
            }

            for edge in graph.edges_directed(node_index, Direction::Outgoing) {
                if visited.insert(edge.target()) {
                    parents.insert(edge.target(), (node_index, edge.id()));
                    stack.push(edge.target());
                }
            }
        }

        None
    }

    /// Remove the serial ordering edges that connect `from` to `to`, as a
    /// dependency is being linked in the other direction, which would otherwise
    /// introduce a cycle. A serial ordering edge is a preference, and is skipped
    /// by [`Self::try_link_edge`] when it contradicts a dependency, but it may
    /// also be linked before the dependency is. This happens when a node is a
    /// cleanup of a node that is created afterwards, or when the dependencies
    /// of tasks inserted without them (`no_dependencies`) are linked once the graph is
    /// built.
    ///
    /// While the nodes are connected, the edge that was linked last (of those in
    /// the path) is removed, as that's the one that would have been skipped if
    /// the dependency had been linked first. Returns false if the nodes are
    /// connected by dependencies alone, which is an actual cycle.
    fn unlink_serial_edges(&mut self, from: NodeIndex, to: NodeIndex) -> bool {
        while let Some(path) = self.find_path(from, to) {
            let Some(edge_index) = path
                .into_iter()
                .filter(|edge_index| self.serial_edges.contains_key(edge_index))
                .max_by_key(|edge_index| self.serial_edges[edge_index])
            else {
                return false;
            };

            self.remove_serial_edge(edge_index);
        }

        true
    }

    fn remove_serial_edge(&mut self, edge_index: EdgeIndex) {
        let Some((index, edge)) = self.graph.edge_endpoints(edge_index) else {
            return;
        };

        trace!(
            index = index.index(),
            requires = ?[edge.index()],
            "Unlinking serial requirements for index, as they contradict a dependency"
        );

        // Removing an edge moves the last edge in the graph into its index
        let last_edge_index = EdgeIndex::new(self.graph.edge_count() - 1);

        self.graph.remove_edge(edge_index);
        self.serial_edges.remove(&edge_index);

        if last_edge_index != edge_index
            && let Some(order) = self.serial_edges.remove(&last_edge_index)
        {
            self.serial_edges.insert(edge_index, order);
        }
    }

    fn is_persistent_index(&self, index: NodeIndex) -> bool {
        self.graph
            .node_weight(index)
            .is_some_and(|node| node.is_persistent())
    }

    /// Try to add a serial ordering edge between two dependency nodes, recording
    /// it in `serial_edges` so the subtree walk in
    /// [`Self::link_serial_required_edges`] won't mistake it for a real
    /// dependency. Silently skips the edge if it would introduce a cycle — this
    /// happens when the same task node appears in multiple serial dependency
    /// chains across different parent tasks, or when the nodes depend on each
    /// other in the opposite order.
    fn try_link_edge(&mut self, index: NodeIndex, edge: NodeIndex) {
        if !self.has_edge_of_type(index, edge, TaskDependencyType::Required)
            && let Ok(edge_index) = self
                .graph
                .add_edge(index, edge, TaskDependencyType::Required)
        {
            self.serial_edges
                .insert(edge_index, self.serial_edges_count);
            self.serial_edges_count += 1;

            trace!(
                index = index.index(),
                requires = ?[edge.index()],
                "Linking requirements for index"
            );
        }
    }

    fn insert_node(&mut self, node: ActionNode) -> NodeIndex {
        let label = node.label();
        let index = self.graph.add_node(node.clone());

        self.nodes.insert(node, index);

        debug!(
            index = index.index(),
            "Adding {} to graph",
            color::muted_light(label)
        );

        index
    }

    fn is_project_affected(
        &mut self,
        project: &Project,
        reqs: &RunRequirements,
    ) -> miette::Result<bool> {
        if let Some(affected) = &mut self.affected
            && !reqs.skip_affected
        {
            // Short-circuit early if the task is already marked
            let marked = if !self.affected_depth.is_none() {
                affected.is_project_marked(project)
            } else {
                affected.is_project_marked_ignoring_relations(project)
            };

            if marked {
                return Ok(true);
            }

            // Otherwise run the full affected checks
            if let Some(mark) = affected.is_project_affected(project) {
                affected.mark_project_affected(project, mark)?;

                return Ok(true);
            }

            return Ok(false);
        }

        // Always affected
        Ok(true)
    }

    fn is_task_affected(&mut self, task: &Task, reqs: &RunRequirements) -> miette::Result<bool> {
        if let Some(affected) = &mut self.affected
            && !reqs.skip_affected
        {
            // Short-circuit early if the task is already marked
            let marked = if !self.affected_depth.is_none() {
                affected.is_task_marked(task)
            } else {
                affected.is_task_marked_ignoring_relations(task)
            };

            if marked {
                return Ok(true);
            }

            // Otherwise run the full affected checks
            if let Some(mark) = affected.is_task_affected(task)? {
                affected.mark_task_affected(task, mark)?;

                return Ok(true);
            }

            return Ok(false);
        }

        // Always affected
        Ok(true)
    }
}

#[cfg(debug_assertions)]
impl ActionGraphBuilder<'_> {
    pub fn mock_affected(
        &mut self,
        changed_files: FxHashSet<WorkspaceRelativePathBuf>,
        depth: Depth,
        mut op: impl FnMut(&mut AffectedTracker),
    ) {
        self.set_changed_files(changed_files).unwrap();
        self.set_affected().unwrap();
        self.affected_depth = depth;

        if let Some(affected) = self.affected.as_mut() {
            affected.set_depth(depth);

            op(affected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_test_utils::WorkspaceMocker;
    use starbase_sandbox::create_sandbox;
    use std::fs;

    fn create_toolchain_spec(id: &str) -> ToolchainSpec {
        ToolchainSpec::new(
            Id::raw(id),
            moon_config::UnresolvedVersionSpec::parse("1.2.3").unwrap(),
        )
    }

    async fn create_builder(root: &std::path::Path) -> ActionGraphBuilder<'static> {
        let mocker = WorkspaceMocker::new(root)
            .load_default_configs()
            .with_all_toolchains()
            .with_test_toolchains()
            .with_default_projects()
            .with_global_envs();

        ActionGraphBuilder::new(
            Arc::new(mocker.mock_app_context()),
            Arc::new(mocker.mock_workspace_graph().await),
            Default::default(),
        )
        .unwrap()
    }

    fn find_node_index(
        graph: &ActionGraph,
        mut predicate: impl FnMut(&ActionNode) -> bool,
    ) -> NodeIndex {
        let inner = graph.get_inner_graph();
        let nodes = graph.get_inner_nodes();

        inner
            .graph()
            .node_indices()
            .find(|index| nodes.get(index).is_some_and(&mut predicate))
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_toolchain_requirements_path_local_across_siblings() {
        let sandbox = create_sandbox("projects");
        let mut builder = create_builder(sandbox.path()).await;
        let id = Id::raw("root");
        let cycle = FxHashSet::from_iter([&id]);

        builder
            .internal_setup_toolchain(&create_toolchain_spec("tc-tier3-reqs"), None, cycle.clone())
            .await
            .unwrap();
        builder
            .internal_setup_toolchain(&create_toolchain_spec("tc-tier2-reqs"), None, cycle.clone())
            .await
            .unwrap();

        let (_, graph) = builder.build();
        let inner = graph.get_inner_graph();
        let child_index = find_node_index(&graph, |node| {
            matches!(
                node,
                ActionNode::SetupToolchain(inner)
                    if inner.toolchain.id.as_str() == "tc-tier2-reqs"
            )
        });
        let shared_index = find_node_index(&graph, |node| {
            matches!(
                node,
                ActionNode::SetupToolchain(inner)
                    if inner.toolchain.id.as_str() == "tc-tier3"
            )
        });

        assert!(inner.find_edge(child_index, shared_index).is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_project_dependencies_path_local_across_siblings() {
        let sandbox = create_sandbox("projects");

        fs::write(
            sandbox.path().join("foo/moon.yml"),
            "dependsOn: [bar, qux]\n",
        )
        .unwrap();
        fs::write(
            sandbox.path().join("bar/moon.yml"),
            "dependsOn: [baz]\nlanguage: javascript\n",
        )
        .unwrap();
        fs::write(
            sandbox.path().join("qux/moon.yml"),
            "dependsOn: [baz]\nlanguage: rust\n\ntoolchains:\n  rust:\n    version: '1.90.0'\n",
        )
        .unwrap();

        let mocker = WorkspaceMocker::new(sandbox.path())
            .load_default_configs()
            .with_all_toolchains()
            .with_test_toolchains()
            .with_default_projects()
            .with_global_envs();
        let workspace_graph = Arc::new(mocker.mock_workspace_graph().await);
        let mut builder = ActionGraphBuilder::new(
            Arc::new(mocker.mock_app_context()),
            Arc::clone(&workspace_graph),
            Default::default(),
        )
        .unwrap();
        let id = Id::raw("root");
        let cycle = FxHashSet::from_iter([&id]);

        builder
            .internal_sync_project(
                &workspace_graph.get_project("bar").unwrap(),
                &RunRequirements::default(),
                cycle.clone(),
            )
            .await
            .unwrap();
        builder
            .internal_sync_project(
                &workspace_graph.get_project("qux").unwrap(),
                &RunRequirements::default(),
                cycle.clone(),
            )
            .await
            .unwrap();

        let (_, graph) = builder.build();
        let inner = graph.get_inner_graph();
        let qux_index = find_node_index(&graph, |node| {
            matches!(
                node,
                ActionNode::SyncProject(inner)
                    if inner.project_id.as_str() == "qux"
            )
        });
        let baz_index = find_node_index(&graph, |node| {
            matches!(
                node,
                ActionNode::SyncProject(inner)
                    if inner.project_id.as_str() == "baz"
            )
        });

        assert!(inner.find_edge(qux_index, baz_index).is_some());
    }
}
