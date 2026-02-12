use crate::action_graph::{ActionGraph, ActionGraphType};
use crate::action_graph_error::ActionGraphError;
use daggy::Dag;
use miette::IntoDiagnostic;
use moon_action::{
    ActionNode, InstallDependenciesNode, RunTaskNode, SetupEnvironmentNode, SetupToolchainNode,
    SyncProjectNode,
};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use moon_app_context::AppContext;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_common::{Id, color};
use moon_config::{EnvMap, PipelineActionSwitch, TaskDependencyConfig, TaskDependencyType};
use moon_pdk_api::{DefineRequirementsInput, LocateDependenciesRootInput};
use moon_project::{Project, ProjectError};
use moon_query::{Criteria, build_query};
use moon_task::{Target, TargetError, TargetLocator, TargetScope, Task};
use moon_toolchain::ToolchainSpec;
use moon_workspace_graph::projects::ProjectGraphError;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Debug;
use std::mem;
use std::sync::Arc;
use tracing::{debug, instrument, trace};

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
    pub ci: bool,                    // Are we in a CI environment
    pub ci_check: bool,              // Check the `runInCI` option
    pub dependencies: UpstreamScope, // Run dependency tasks
    pub dependents: DownstreamScope, // Run dependent tasks
    pub interactive: bool,           // Entire pipeline is interactive
    pub job: Option<usize>,          // Current job index
    pub job_total: Option<usize>,    // Total amount of jobs
    pub skip_affected: bool,         // Skip all affected checks
}

impl Default for RunRequirements {
    fn default() -> Self {
        Self {
            ci: false,
            ci_check: false,
            dependencies: UpstreamScope::Deep,
            dependents: DownstreamScope::None,
            interactive: false,
            job: None,
            job_total: None,
            skip_affected: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct RunPartition {
    pub targets: FxHashMap<NodeIndex, Target>,
    pub size: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct RunTaskState {
    pub depth: u8,
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
    graph: ActionGraphType,
    nodes: FxHashMap<ActionNode, NodeIndex>,
    options: ActionGraphBuilderOptions,
    workspace_graph: Arc<WorkspaceGraph>,

    // Affected tracking
    affected: Option<AffectedTracker>,
    changed_files: Option<FxHashSet<WorkspaceRelativePathBuf>>,

    // Target tracking
    // initial_targets: FxHashSet<Target>,
    passthrough_targets: FxHashSet<Target>,
    primary_targets: FxHashSet<Target>,
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
            passthrough_targets: FxHashSet::default(),
            primary_targets: FxHashSet::default(),
            changed_files: None,
            workspace_graph,
        })
    }

    pub fn build(mut self) -> (ActionContext, ActionGraph) {
        let mut context = ActionContext {
            affected: self.affected.take().map(|affected| affected.build()),
            ..ActionContext::default()
        };

        if !self.passthrough_targets.is_empty() {
            for target in mem::take(&mut self.passthrough_targets) {
                context.set_target_state(target, TargetState::Passthrough);
            }
        }

        if !self.primary_targets.is_empty() {
            context.primary_targets = mem::take(&mut self.primary_targets);
        }

        if let Some(files) = self.changed_files.take() {
            context.changed_files = files.to_owned();
        }

        // Reduce unncessary edges
        if let Some(index) = self.get_index_from_node(&ActionNode::SyncWorkspace) {
            self.graph.transitive_reduce(vec![index]);
        }

        (context, ActionGraph::new(self.graph))
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

    pub fn track_affected(
        &mut self,
        upstream: UpstreamScope,
        downstream: DownstreamScope,
        ci_check: bool,
    ) -> miette::Result<()> {
        // If we require dependents, then we must load all projects into the
        // graph so that the edges are created!
        if downstream != DownstreamScope::None {
            debug!("Force loading all projects and tasks to determine relationships");

            self.workspace_graph.get_projects()?;
            self.workspace_graph.get_tasks_with_internal()?;
        }

        self.set_affected()?;

        if let Some(affected) = self.affected.as_mut() {
            affected.set_ci_check(ci_check);
            affected.with_scopes(upstream, downstream);
            affected.track_projects()?;
            affected.track_tasks()?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: &Project,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.install_dependencies.is_enabled(&spec.id) || spec.is_system() {
            return Ok(None);
        }

        let sync_workspace_index = self.sync_workspace().await?;
        let setup_toolchain_index = self.setup_toolchain(spec, Some(project)).await?;
        let toolchain_registry = &self.app_context.toolchain_registry;
        let toolchain = toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support this action, so skip and fall through
        if !toolchain.supports_tier_2().await {
            return Ok(setup_toolchain_index);
        }

        let output = toolchain
            .locate_dependencies_root(LocateDependenciesRootInput {
                context: toolchain_registry.create_context(),
                starting_dir: toolchain.to_virtual_path(&project.root),
                toolchain_config: toolchain_registry
                    .create_merged_config(&toolchain.id, &project.config),
            })
            .await?;

        // Only insert this action if a root was located
        if let Some(abs_root) = output.root.as_ref() {
            let rel_root = abs_root
                .relative_to(&self.app_context.workspace_root)
                .into_diagnostic()?;

            // Determine if we're in the dependencies workspace
            let in_workspace = toolchain.in_dependencies_workspace(&output, &project.root)?;

            // If not in the dependencies workspace (if there is one),
            // or is a stand-alone project with its own lockfile,
            // we must extract the project ID and source (root)
            let (project_id, root) = if in_workspace {
                (None, rel_root)
            } else {
                (Some(project.id.clone()), project.source.clone())
            };

            let setup_env_index = self
                .setup_environment(spec, &root, project_id.as_ref().map(|_| project))
                .await?;

            // Only create this action if the plugin supports it
            if toolchain.has_func("install_dependencies").await {
                let index = insert_node_if_missing!(
                    self,
                    ActionNode::install_dependencies(InstallDependenciesNode {
                        members: if in_workspace { output.members } else { None },
                        project_id,
                        root,
                        toolchain_id: spec.id.clone(),
                    })
                );

                self.link_first_requirement(
                    index,
                    vec![setup_env_index, setup_toolchain_index, sync_workspace_index],
                )?;

                return Ok(Some(index));
            }

            // Otherwise pass through to setup environment
            if let Some(setup_env_index) = setup_env_index {
                self.link_first_requirement(
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
    pub async fn run_task(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Option<NodeIndex>> {
        if let Some(index) =
            Box::pin(self.internal_run_task(task, reqs, None, &mut RunTaskState::default())).await?
        {
            // Only track primary targets at the top-level run methods,
            // as these are explicitly called by pipeline consumers!
            self.primary_targets.insert(task.target.clone());

            return Ok(Some(index));
        }

        Ok(None)
    }

    #[cfg(debug_assertions)]
    pub async fn run_task_with_config(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        config: &TaskDependencyConfig,
    ) -> miette::Result<Option<NodeIndex>> {
        Box::pin(self.internal_run_task(task, reqs, Some(config), &mut RunTaskState::default()))
            .await
    }

    #[instrument(skip(self))]
    pub async fn run_task_by_target<T: AsRef<Target> + Debug>(
        &mut self,
        target: T,
        reqs: &RunRequirements,
    ) -> miette::Result<FxHashSet<NodeIndex>> {
        let target = target.as_ref();
        let mut indexes = FxHashSet::default();

        for task in self
            .internal_resolve_tasks_from_target(target, false)
            .await?
        {
            if let Some(index) = self.run_task(&task, reqs).await? {
                indexes.insert(index);
            }
        }

        Ok(indexes)
    }

    #[instrument(skip(self))]
    pub async fn run_task_by_target_locator<T: AsRef<TargetLocator> + Debug>(
        &mut self,
        locator: T,
        reqs: &RunRequirements,
    ) -> miette::Result<FxHashSet<NodeIndex>> {
        let locator = locator.as_ref();
        let mut indexes = FxHashSet::default();

        for task in self
            .internal_resolve_tasks_from_target_locator(locator, false)
            .await?
        {
            if let Some(index) = self.run_task(&task, reqs).await? {
                indexes.insert(index);
            }
        }

        Ok(indexes)
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

        // Now partition the tasks list based on the job information
        if let Some(job_index) = reqs.job
            && let Some(job_total) = reqs.job_total
            && job_total > 0
        {
            // If we are going to parallelize, then we need to filter the
            // tasks list based on affected state before partitioning!
            if !reqs.skip_affected
                && let Some(affected) = &self.affected
            {
                tasks.retain(|task| affected.is_task_marked_ignoring_relations(task));
            }

            let size = tasks.len().div_ceil(job_total);
            let (start, stop) =
                // beginning
                if job_index == 0 {
                    (0, size)
                }
                // end
                else if job_index == job_total - 1 {
                    ((size * job_index), tasks.len())
                }
                // middle
                else {
                    ((size * job_index), (size * (job_index + 1)))
                };

            if tasks.get(start).is_some() {
                if tasks.get(stop).is_some() {
                    tasks = tasks[start..stop].to_vec();
                } else {
                    tasks = tasks[start..].to_vec();
                }
            }

            partition.size = Some(size);
        }

        for task in tasks {
            if let Some(index) = self.run_task(&task, &reqs).await? {
                partition.targets.insert(index, task.target.clone());
            }
        }

        Ok(partition)
    }

    #[instrument(skip(self))]
    pub async fn run_task_dependencies(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        state: &RunTaskState,
    ) -> miette::Result<Vec<Option<NodeIndex>>> {
        let parallel = task.options.run_deps_in_parallel;
        let mut indexes: Vec<Option<NodeIndex>> = vec![];
        let mut previous_target_index: Option<NodeIndex> = None;

        for dep in &task.deps {
            for dep_task in self
                .internal_resolve_tasks_from_target(&dep.target, true)
                .await?
            {
                if let Some(dep_index) =
                    Box::pin(self.internal_run_task(&dep_task, reqs, Some(dep), &mut state.clone()))
                        .await?
                {
                    // When parallel, parent depends on child
                    if parallel {
                        indexes.push(Some(dep_index));
                    }
                    // When serial, next child depends on previous child
                    else if let Some(prev) = previous_target_index {
                        self.link_requirements(dep_index, vec![prev])?;
                    }

                    previous_target_index = Some(dep_index);
                }
            }
        }

        if !parallel {
            indexes.push(previous_target_index);
        }

        Ok(indexes)
    }

    #[instrument(skip(self))]
    pub async fn run_task_dependents(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
        state: &RunTaskState,
    ) -> miette::Result<Vec<Option<NodeIndex>>> {
        let mut indexes = vec![];

        for dep_target in self.workspace_graph.tasks.dependents_of(task) {
            for dep_task in self
                .internal_resolve_tasks_from_target(&dep_target, true)
                .await?
            {
                indexes.push(
                    Box::pin(self.internal_run_task(&dep_task, reqs, None, &mut state.clone()))
                        .await?,
                );
            }
        }

        Ok(indexes)
    }

    #[instrument(skip(self))]
    async fn internal_resolve_tasks_from_target(
        &mut self,
        target: &Target,
        allow_internal: bool,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];

        match &target.scope {
            // :task
            TargetScope::All => {
                let mut projects = vec![];

                if let Some(all_query) = &self.all_query {
                    projects.extend(self.workspace_graph.query_projects(all_query)?);
                } else {
                    projects.extend(self.workspace_graph.get_projects()?);
                };

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = self
                        .workspace_graph
                        .get_task_from_project(&project.id, &target.task_id)
                    {
                        if !allow_internal && task.is_internal() {
                            continue;
                        }

                        tasks.push(task);
                    }
                }
            }
            // ^:task
            TargetScope::Deps => {
                return Err(TargetError::NoDepsInRunContext.into());
            }
            // project:task
            TargetScope::Project(project_id) => {
                let task = self
                    .workspace_graph
                    .get_task_from_project(project_id, &target.task_id)?;

                // Don't allow internal tasks to be ran
                if !allow_internal && task.is_internal() {
                    return Err(ProjectError::UnknownTask {
                        task_id: task.id.to_string(),
                        project_id: project_id.to_string(),
                    }
                    .into());
                }

                tasks.push(task);
            }
            // #tag:task
            TargetScope::Tag(tag) => {
                let projects = self
                    .workspace_graph
                    .query_projects(build_query(format!("tag={tag}").as_str())?)?;

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = self
                        .workspace_graph
                        .get_task_from_project(&project.id, &target.task_id)
                    {
                        if !allow_internal && task.is_internal() {
                            continue;
                        }

                        tasks.push(task);
                    }
                }
            }
            // ~:task
            TargetScope::OwnSelf => {
                return Err(TargetError::NoSelfInRunContext.into());
            }
        };

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
                scope,
                scope_glob,
                task_glob,
                ..
            } => {
                let mut is_all = false;
                let mut do_query = false;
                let mut projects = vec![];

                // Query for all applicable projects first since we can't
                // query projects + tasks at the same time
                if let Some(glob) = scope_glob {
                    let query = if let Some(tag_glob) = glob.strip_prefix('#') {
                        format!("tag~{tag_glob}")
                    } else {
                        format!("project~{glob}")
                    };

                    projects = self.workspace_graph.query_projects(build_query(&query)?)?;
                    do_query = !projects.is_empty();
                } else {
                    match scope {
                        Some(TargetScope::All) => {
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
                    let mut query = format!("task~{task_glob}");

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
                let target = if target.scope == TargetScope::OwnSelf {
                    Target::new(
                        &self.workspace_graph.get_project_from_path(None)?.id,
                        &target.task_id,
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
        state: &mut RunTaskState,
    ) -> miette::Result<Option<NodeIndex>> {
        let project = self
            .workspace_graph
            .get_project(task.target.get_project_id()?)?;
        let mut child_reqs = reqs.clone();

        // Abort early if not affected
        if let Some(affected) = &mut self.affected
            && !reqs.skip_affected
            && !affected.is_task_marked_ignoring_relations(task)
        {
            return Ok(None);
        }

        // These tasks shouldn't actually run, so filter them out
        if self.passthrough_targets.contains(&task.target) {
            trace!(
                task_target = task.target.as_str(),
                "Not running task {} because it has been marked as passthrough",
                color::id(&task.target.id),
            );

            return Ok(None);
        }

        // Track depth information before proceeding
        let should_run_dependencies = reqs.dependencies.is_in_scope(state.depth);
        let should_run_dependents = reqs.dependents.is_in_scope(state.depth);
        state.depth += 1;

        // Only apply checks when requested. This applies to `moon ci`,
        // but not `moon run`, since the latter should be able to
        // manually run local tasks in CI (deploys, etc).
        if reqs.ci && reqs.ci_check && !task.should_run_in_ci() {
            self.passthrough_targets.insert(task.target.clone());

            debug!(
                task_target = task.target.as_str(),
                "Not running task {} in CI because {} has been configured not to",
                color::id(&task.target.id),
                color::property("runInCI"),
            );

            // Dependents may still want to run though!
            if should_run_dependents {
                child_reqs.skip_affected = false;

                Box::pin(self.run_task_dependents(task, &child_reqs, state)).await?;
            }

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
        let mut edges = vec![self.sync_project(&project).await?];

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

        if !task.deps.is_empty() && should_run_dependencies {
            child_reqs.skip_affected = true;

            edges.extend(Box::pin(self.run_task_dependencies(task, &child_reqs, state)).await?);
        }

        self.link_optional_requirements(index, edges)?;

        // And possibly dependents
        if should_run_dependents {
            child_reqs.skip_affected = false;

            Box::pin(self.run_task_dependents(task, &child_reqs, state)).await?;
        }

        Ok(Some(index))
    }

    #[instrument(skip(self))]
    pub async fn setup_environment(
        &mut self,
        spec: &ToolchainSpec,
        root: &WorkspaceRelativePathBuf,
        project: Option<&Project>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.setup_environment.is_enabled(&spec.id) || spec.is_system() {
            return Ok(None);
        }

        let toolchain = self.app_context.toolchain_registry.load(&spec.id).await?;

        // Toolchain does not support it
        if !toolchain.has_func("setup_environment").await {
            return Ok(None);
        }

        let sync_workspace_index = self.sync_workspace().await?;
        let setup_toolchain_index = self.setup_toolchain(spec, project).await?;

        let index = insert_node_or_exit!(
            self,
            ActionNode::setup_environment(SetupEnvironmentNode {
                project_id: project.map(|p| p.id.clone()),
                root: root.clone(),
                toolchain_id: spec.id.clone(),
            })
        );

        self.link_first_requirement(index, vec![setup_toolchain_index, sync_workspace_index])?;

        Ok(Some(index))
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
        Box::pin(self.internal_setup_toolchain(spec, project, &mut FxHashSet::default())).await
    }

    async fn internal_setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
        cycle: &mut FxHashSet<Id>,
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

        cycle.insert(spec.id.clone());

        // Toolchain may depend on others
        if toolchain.has_func("define_requirements").await {
            let output = toolchain
                .define_requirements(DefineRequirementsInput {
                    context: toolchain_registry.create_context(),
                    toolchain_config: toolchain_registry.create_config(&toolchain.id),
                })
                .await?;

            if !output.requires.is_empty() {
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
                                    cycle,
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

        self.link_optional_requirements(index, edges)?;

        Ok(Some(index))
    }

    #[instrument(skip(self))]
    pub async fn sync_project(&mut self, project: &Project) -> miette::Result<Option<NodeIndex>> {
        Box::pin(self.internal_sync_project(project, &mut FxHashSet::default())).await
    }

    async fn internal_sync_project(
        &mut self,
        project: &Project,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Explicitly disabled
        if !self.options.sync_projects.is_enabled(&project.id) {
            return Ok(None);
        }

        // Return early if not affected
        if let Some(affected) = &mut self.affected
            && !affected.is_project_marked(project)
        {
            return Ok(None);
        }

        // Insert the node and edges
        let mut edges = vec![];

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
            cycle.insert(project.id.clone());

            for dep_project_id in self.workspace_graph.projects.dependencies_of(project) {
                if cycle.contains(&dep_project_id) {
                    continue;
                }

                let dep_project = self.workspace_graph.get_project(&dep_project_id)?;

                if let Some(dep_project_index) =
                    Box::pin(self.internal_sync_project(&dep_project, cycle)).await?
                    && index != dep_project_index
                {
                    edges.push(dep_project_index);
                }
            }
        }

        if !edges.is_empty() {
            self.link_requirements(index, edges)?;
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

    fn get_index_from_node(&self, node: &ActionNode) -> Option<NodeIndex> {
        self.nodes.get(node).cloned()
    }

    fn link_first_requirement(
        &mut self,
        index: NodeIndex,
        edges: Vec<Option<NodeIndex>>,
    ) -> miette::Result<()> {
        if let Some(edge) = edges.into_iter().flatten().next() {
            self.link_requirements(index, vec![edge])?;
        }

        Ok(())
    }

    fn link_optional_requirements(
        &mut self,
        index: NodeIndex,
        edges: Vec<Option<NodeIndex>>,
    ) -> miette::Result<()> {
        self.link_requirements(index, edges.into_iter().flatten().collect())
    }

    fn link_requirements(&mut self, index: NodeIndex, edges: Vec<NodeIndex>) -> miette::Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        let mut added_edges = vec![];

        for edge in edges {
            if self.graph.find_edge(index, edge).is_none() {
                self.graph
                    .add_edge(index, edge, TaskDependencyType::Required)
                    .map_err(|_| ActionGraphError::WouldCycle {
                        source_action: self.graph.node_weight(index).unwrap().label(),
                        target_action: self.graph.node_weight(edge).unwrap().label(),
                    })?;

                added_edges.push(edge);
            }
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
}

#[cfg(debug_assertions)]
impl ActionGraphBuilder<'_> {
    pub fn mock_affected(
        &mut self,
        changed_files: FxHashSet<WorkspaceRelativePathBuf>,
        mut op: impl FnMut(&mut AffectedTracker),
    ) {
        self.set_changed_files(changed_files).unwrap();
        self.set_affected().unwrap();

        if let Some(affected) = self.affected.as_mut() {
            op(affected);
        }
    }
}
