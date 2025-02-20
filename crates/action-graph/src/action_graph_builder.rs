use crate::action_graph::ActionGraph;
use moon_action::{
    ActionNode, InstallProjectDepsNode, InstallWorkspaceDepsNode, RunTaskNode, SetupToolchainNode,
    SyncProjectNode,
};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{color, Id};
use moon_config::TaskDependencyConfig;
use moon_platform::{PlatformManager, Runtime};
use moon_project::Project;
use moon_query::{build_query, Criteria};
use moon_task::{Target, TargetError, TargetLocator, TargetScope, Task};
use moon_task_args::parse_task_args;
use moon_workspace_graph::{tasks::TaskGraphError, GraphConnections, WorkspaceGraph};
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::mem;
use tracing::{debug, instrument, trace};

#[derive(Default)]
pub struct RunRequirements {
    pub ci: bool,         // Are we in a CI environment
    pub ci_check: bool,   // Check the `runInCI` option
    pub dependents: bool, // Run dependent tasks as well
    pub interactive: bool,
    pub skip_affected: bool, // Temporary until we support task dependents properly
    pub target_locators: FxHashSet<TargetLocator>,
}

impl RunRequirements {
    pub fn has_target(&self, target: &Target) -> bool {
        self.target_locators.iter().any(|loc| loc == target)
    }
}

pub struct ActionGraphBuilder<'app> {
    all_query: Option<Criteria<'app>>,
    graph: DiGraph<ActionNode, ()>,
    indices: FxHashMap<ActionNode, NodeIndex>,
    platform_manager: &'app PlatformManager,
    workspace_graph: &'app WorkspaceGraph,

    // Affected states
    affected: Option<AffectedTracker<'app>>,
    touched_files: Option<FxHashSet<WorkspaceRelativePathBuf>>,

    // Target tracking
    initial_targets: FxHashSet<Target>,
    passthrough_targets: FxHashSet<Target>,
    primary_targets: FxHashSet<Target>,
}

impl<'app> ActionGraphBuilder<'app> {
    pub fn new(workspace_graph: &'app WorkspaceGraph) -> miette::Result<Self> {
        ActionGraphBuilder::with_platforms(PlatformManager::read(), workspace_graph)
    }

    pub fn with_platforms(
        platform_manager: &'app PlatformManager,
        workspace_graph: &'app WorkspaceGraph,
    ) -> miette::Result<Self> {
        debug!("Building action graph");

        Ok(ActionGraphBuilder {
            all_query: None,
            affected: None,
            graph: DiGraph::new(),
            indices: FxHashMap::default(),
            initial_targets: FxHashSet::default(),
            passthrough_targets: FxHashSet::default(),
            platform_manager,
            primary_targets: FxHashSet::default(),
            workspace_graph,
            touched_files: None,
        })
    }

    pub fn build(self) -> ActionGraph {
        ActionGraph::new(self.graph)
    }

    pub fn build_context(&mut self) -> ActionContext {
        let mut context = ActionContext {
            affected: self.affected.take().map(|affected| affected.build()),
            ..ActionContext::default()
        };

        if !self.initial_targets.is_empty() {
            context.initial_targets = mem::take(&mut self.initial_targets);
        }

        if !self.passthrough_targets.is_empty() {
            for target in mem::take(&mut self.passthrough_targets) {
                context.set_target_state(target, TargetState::Passthrough);
            }
        }

        if !self.primary_targets.is_empty() {
            context.primary_targets = mem::take(&mut self.primary_targets);
        }

        if let Some(files) = self.touched_files.take() {
            context.touched_files = files.to_owned();
        }

        context
    }

    pub fn get_index_from_node(&self, node: &ActionNode) -> Option<&NodeIndex> {
        self.indices.get(node)
    }

    pub fn get_runtime(&self, project: &Project, toolchain: &Id, allow_override: bool) -> Runtime {
        if let Ok(platform) = self.platform_manager.get_by_toolchain(toolchain) {
            return platform.get_runtime_from_config(if allow_override {
                Some(&project.config)
            } else {
                None
            });
        }

        Runtime::system()
    }

    pub fn set_affected(&mut self) {
        let Some(touched_files) = &self.touched_files else {
            return;
        };

        if self.affected.is_none() {
            self.affected = Some(AffectedTracker::new(
                self.workspace_graph,
                touched_files.to_owned(),
            ));
        }
    }

    pub fn set_affected_scopes(
        &mut self,
        upstream: UpstreamScope,
        downstream: DownstreamScope,
    ) -> miette::Result<()> {
        // If we require dependents, then we must load all projects into the
        // graph so that the edges are created!
        if downstream != DownstreamScope::None {
            debug!("Force loading all projects and tasks to determine relationships");

            self.workspace_graph.get_projects()?;
            self.workspace_graph.get_tasks_with_internal()?;
        }

        self.set_affected();
        self.affected
            .as_mut()
            .unwrap()
            .with_scopes(upstream, downstream);

        Ok(())
    }

    pub fn set_query(&mut self, input: &'app str) -> miette::Result<()> {
        self.all_query = Some(build_query(input)?);

        Ok(())
    }

    pub fn set_touched_files(
        &mut self,
        touched_files: FxHashSet<WorkspaceRelativePathBuf>,
    ) -> miette::Result<()> {
        self.touched_files = Some(touched_files);

        Ok(())
    }

    // ACTIONS

    #[instrument(skip_all)]
    pub fn install_deps(
        &mut self,
        project: &Project,
        task: Option<&Task>,
    ) -> miette::Result<Option<NodeIndex>> {
        let mut in_project = false;
        let toolchains = task
            .map(|t| &t.toolchains)
            .unwrap_or_else(|| &project.toolchains);
        let mut primary_toolchain = toolchains[0].to_owned();
        let mut packages_root = WorkspaceRelativePathBuf::default();

        // If Bun and Node.js are enabled, they will both attempt to install
        // dependencies in the target root. We need to avoid this problem,
        // so always prefer Node.js instead. Revisit in the future.
        if primary_toolchain == "bun"
            && self
                .platform_manager
                .enabled()
                .any(|enabled_platform| enabled_platform == "node")
        {
            debug!(
                "Already installing dependencies with node, skipping a conflicting install from bun"
            );

            primary_toolchain = Id::raw("node")
        }

        // If project is NOT in the package manager workspace, then we should
        // install dependencies in the project, not the workspace root.
        if let Ok(platform) = self.platform_manager.get_by_toolchain(&primary_toolchain) {
            packages_root = platform.find_dependency_workspace_root(project.source.as_str())?;

            if !platform
                .is_project_in_dependency_workspace(&packages_root, project.source.as_str())?
            {
                in_project = true;

                debug!(
                    "Project {} is not within the dependency manager workspace, dependencies will be installed within the project instead of the root",
                    color::id(&project.id),
                );
            }
        }

        let node = if in_project {
            ActionNode::install_project_deps(InstallProjectDepsNode {
                project_id: project.id.to_owned(),
                runtime: self.get_runtime(project, &primary_toolchain, true),
            })
        } else {
            ActionNode::install_workspace_deps(InstallWorkspaceDepsNode {
                runtime: self.get_runtime(project, &primary_toolchain, false),
                root: packages_root,
            })
        };

        if node.get_runtime().is_system() {
            return Ok(None);
        }

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        // Before we install deps, we must ensure the language has been installed
        let setup_tool_index = self.setup_toolchain(node.get_runtime());
        let index = self.insert_node(node);

        self.link_requirements(index, vec![setup_tool_index]);

        Ok(Some(index))
    }

    pub fn run_task(
        &mut self,
        project: &Project,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Option<NodeIndex>> {
        self.run_task_with_config(project, task, reqs, None)
    }

    #[instrument(skip_all)]
    pub fn run_task_with_config(
        &mut self,
        project: &Project,
        task: &Task,
        reqs: &RunRequirements,
        config: Option<&TaskDependencyConfig>,
    ) -> miette::Result<Option<NodeIndex>> {
        // Create a new requirements object as we don't want our dependencies/
        // dependents to check for affected or run their own dependents!
        let child_reqs = RunRequirements {
            ci: reqs.ci,
            ci_check: reqs.ci_check,
            dependents: false,
            interactive: reqs.interactive,
            skip_affected: true,
            ..Default::default()
        };

        // Only apply checks when requested. This applies to `moon ci`,
        // but not `moon run`, since the latter should be able to
        // manually run local tasks in CI (deploys, etc).
        if reqs.ci && reqs.ci_check && !task.should_run_in_ci() {
            self.passthrough_targets.insert(task.target.clone());

            debug!(
                task_target = task.target.as_str(),
                "Not running task {} because {} is false",
                color::label(&task.target.id),
                color::property("runInCI"),
            );

            // Dependents may still want to run though,
            // but only if this task was affected
            if reqs.dependents {
                if let Some(affected) = &mut self.affected {
                    trace!(
                        task_target = task.target.as_str(),
                        "But will run all dependent tasks if affected"
                    );

                    if affected.is_task_marked(task) {
                        self.run_task_dependents(task, &child_reqs)?;
                    }
                }
            }

            return Ok(None);
        }

        // These tasks shouldn't actually run, so filter them out
        if self.passthrough_targets.contains(&task.target) {
            trace!(
                task_target = task.target.as_str(),
                "Not adding task {} to graph because it has been marked as passthrough",
                color::label(&task.target.id),
            );

            return Ok(None);
        }

        let mut args = vec![];
        let mut env = FxHashMap::default();

        if let Some(config) = config {
            args.extend(parse_task_args(&config.args)?);
            env.extend(config.env.clone());
        }

        let node = ActionNode::run_task(RunTaskNode {
            args,
            env,
            interactive: task.is_interactive() || reqs.interactive,
            persistent: task.is_persistent(),
            runtime: self.get_runtime(project, &task.toolchains[0], true),
            target: task.target.to_owned(),
            id: None,
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(Some(*index));
        }

        // Compare against touched files if provided
        if let Some(affected) = &mut self.affected {
            if !reqs.skip_affected && !affected.is_task_marked(task) {
                return Ok(None);
            }
        }

        // We should install deps & sync projects *before* running targets
        let mut edges = vec![];

        if let Some(install_deps_index) = self.install_deps(project, Some(task))? {
            edges.push(install_deps_index);
        }

        edges.push(self.sync_project(project)?);

        // Insert the node and create edges
        let index = self.insert_node(node);

        // And we also need to create edges for task dependencies
        if !task.deps.is_empty() {
            trace!(
                task_target = task.target.as_str(),
                dep_targets = ?task.deps.iter().map(|d| d.target.as_str()).collect::<Vec<_>>(),
                "Linking dependencies for task",
            );

            edges.extend(self.run_task_dependencies(task, &child_reqs)?);
        }

        self.link_requirements(index, edges);

        // And possibly dependents
        if reqs.dependents {
            self.run_task_dependents(task, &child_reqs)?;
        }

        Ok(Some(index))
    }

    // We don't pass touched files to dependencies, because if the parent
    // task is affected/going to run, then so should all of these!
    #[instrument(skip_all)]
    pub fn run_task_dependencies(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Vec<NodeIndex>> {
        let parallel = task.options.run_deps_in_parallel;
        let mut indices = vec![];
        let mut previous_target_index = None;

        for dep in &task.deps {
            let (_, dep_indices) =
                self.run_task_by_target_with_config(&dep.target, reqs, Some(dep))?;

            for dep_index in dep_indices {
                // When parallel, parent depends on child
                if parallel {
                    indices.push(dep_index);

                    // When serial, next child depends on previous child
                } else if let Some(prev) = previous_target_index {
                    self.link_requirements(dep_index, vec![prev]);
                }

                previous_target_index = Some(dep_index);
            }
        }

        if !parallel {
            if let Some(index) = previous_target_index {
                indices.push(index);
            }
        }

        Ok(indices)
    }

    #[instrument(skip_all)]
    pub fn run_task_dependents(
        &mut self,
        task: &Task,
        reqs: &RunRequirements,
    ) -> miette::Result<Vec<NodeIndex>> {
        let mut indices = vec![];

        for dep_target in self.workspace_graph.tasks.dependents_of(task) {
            let (_, dep_indices) = self.run_task_by_target(dep_target, reqs)?;

            for dep_index in dep_indices {
                indices.push(dep_index);
            }
        }

        Ok(indices)
    }

    pub fn run_task_by_target<T: AsRef<Target>>(
        &mut self,
        target: T,
        reqs: &RunRequirements,
    ) -> miette::Result<(FxHashSet<Target>, FxHashSet<NodeIndex>)> {
        self.run_task_by_target_with_config(target, reqs, None)
    }

    pub fn run_task_by_target_with_config<T: AsRef<Target>>(
        &mut self,
        target: T,
        reqs: &RunRequirements,
        config: Option<&TaskDependencyConfig>,
    ) -> miette::Result<(FxHashSet<Target>, FxHashSet<NodeIndex>)> {
        let target = target.as_ref();
        let mut inserted_targets = FxHashSet::default();
        let mut inserted_indices = FxHashSet::default();

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
                        if task.is_internal() {
                            continue;
                        }

                        if let Some(index) =
                            self.run_task_with_config(&project, &task, reqs, config)?
                        {
                            inserted_targets.insert(task.target.clone());
                            inserted_indices.insert(index);
                        }
                    }
                }
            }
            // ^:task
            TargetScope::Deps => {
                return Err(TargetError::NoDepsInRunContext.into());
            }
            // project:task
            TargetScope::Project(project_locator) => {
                let project = self.workspace_graph.get_project(project_locator)?;
                let task = self
                    .workspace_graph
                    .get_task_from_project(&project.id, &target.task_id)?;

                // Don't allow internal tasks to be ran
                if task.is_internal() && reqs.has_target(&task.target) {
                    return Err(TaskGraphError::UnconfiguredTarget(task.target.clone()).into());
                }

                if let Some(index) = self.run_task_with_config(&project, &task, reqs, config)? {
                    inserted_targets.insert(task.target.to_owned());
                    inserted_indices.insert(index);
                }
            }
            // #tag:task
            TargetScope::Tag(tag) => {
                let projects = self
                    .workspace_graph
                    .query_projects(build_query(format!("tag={}", tag).as_str())?)?;

                for project in projects {
                    // Don't error if the task does not exist
                    if let Ok(task) = self
                        .workspace_graph
                        .get_task_from_project(&project.id, &target.task_id)
                    {
                        if task.is_internal() {
                            continue;
                        }

                        if let Some(index) =
                            self.run_task_with_config(&project, &task, reqs, config)?
                        {
                            inserted_targets.insert(task.target.clone());
                            inserted_indices.insert(index);
                        }
                    }
                }
            }
            // ~:task
            TargetScope::OwnSelf => {
                return Err(TargetError::NoSelfInRunContext.into());
            }
        };

        Ok((inserted_targets, inserted_indices))
    }

    #[instrument(skip_all)]
    pub fn run_from_requirements(
        &mut self,
        reqs: RunRequirements,
    ) -> miette::Result<Vec<NodeIndex>> {
        let mut inserted_nodes = vec![];
        let mut initial_targets = vec![];

        if let Some(affected) = &mut self.affected {
            affected.set_ci_check(reqs.ci_check);
        }

        // Track the qualified as an initial target
        for locator in reqs.target_locators.clone() {
            match locator {
                TargetLocator::GlobMatch {
                    project_glob,
                    task_glob,
                    scope,
                    ..
                } => {
                    let mut is_all = false;
                    let mut do_query = false;
                    let mut projects = vec![];

                    // Query for all applicable projects first since we can't
                    // query projects + tasks at the same time
                    if let Some(glob) = project_glob {
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

                        let tasks = self.workspace_graph.query_tasks(build_query(&query)?)?;

                        initial_targets.extend(
                            tasks
                                .into_iter()
                                .map(|task| task.target.clone())
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                TargetLocator::Qualified(target) => {
                    initial_targets.push(target);
                }
                TargetLocator::TaskFromWorkingDir(task_id) => {
                    initial_targets.push(Target::new(
                        &self.workspace_graph.get_project_from_path(None)?.id,
                        task_id,
                    )?);
                }
            };
        }

        // Determine affected tasks before building
        if let Some(affected) = &mut self.affected {
            affected.track_tasks()?;
        }

        // Then build and track initial and primary
        for target in initial_targets {
            let (inserted_targets, inserted_indices) = self.run_task_by_target(&target, &reqs)?;

            self.initial_targets.insert(target.clone());
            self.primary_targets.extend(inserted_targets);

            inserted_nodes.extend(inserted_indices);
        }

        Ok(inserted_nodes)
    }

    #[instrument(skip_all)]
    pub fn setup_toolchain(&mut self, runtime: &Runtime) -> NodeIndex {
        let node = ActionNode::setup_toolchain(SetupToolchainNode {
            runtime: runtime.to_owned(),
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        let sync_workspace_index = self.sync_workspace();
        let index = self.insert_node(node);

        self.link_requirements(index, vec![sync_workspace_index]);

        index
    }

    #[instrument(skip_all)]
    pub fn sync_project(&mut self, project: &Project) -> miette::Result<NodeIndex> {
        self.internal_sync_project(project, &mut FxHashSet::default())
    }

    fn internal_sync_project(
        &mut self,
        project: &Project,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<NodeIndex> {
        let node = ActionNode::sync_project(SyncProjectNode {
            project_id: project.id.clone(),
            runtime: self.get_runtime(project, &project.toolchains[0], true),
        });

        if let Some(index) = self.get_index_from_node(&node) {
            return Ok(*index);
        }

        cycle.insert(project.id.clone());

        // Determine affected state
        if let Some(affected) = &mut self.affected {
            if let Some(by) = affected.is_project_affected(project) {
                affected.mark_project_affected(project, by)?;
            }
        }

        // Syncing requires the language's tool to be installed
        let setup_tool_index = self.setup_toolchain(node.get_runtime());
        let index = self.insert_node(node);
        let mut edges = vec![setup_tool_index];

        // And we should also depend on other projects
        for dep_project_id in self.workspace_graph.projects.dependencies_of(project) {
            if cycle.contains(&dep_project_id) {
                continue;
            }

            let dep_project = self.workspace_graph.get_project(&dep_project_id)?;
            let dep_project_index = self.internal_sync_project(&dep_project, cycle)?;

            if index != dep_project_index {
                edges.push(dep_project_index);
            }
        }

        self.link_requirements(index, edges);

        Ok(index)
    }

    pub fn sync_workspace(&mut self) -> NodeIndex {
        let node = ActionNode::sync_workspace();

        if let Some(index) = self.get_index_from_node(&node) {
            return *index;
        }

        self.insert_node(node)
    }

    // PRIVATE

    fn link_requirements(&mut self, index: NodeIndex, edges: Vec<NodeIndex>) {
        trace!(
            index = index.index(),
            requires = ?edges.iter().map(|i| i.index()).collect::<Vec<_>>(),
            "Linking requirements for index"
        );

        for edge in edges {
            self.graph.add_edge(index, edge, ());
        }
    }

    fn insert_node(&mut self, node: ActionNode) -> NodeIndex {
        let index = self.graph.add_node(node.clone());

        debug!(
            index = index.index(),
            "Adding {} to graph",
            color::muted_light(node.label())
        );

        self.indices.insert(node, index);

        index
    }
}

#[cfg(debug_assertions)]
impl<'app> ActionGraphBuilder<'app> {
    pub fn mock_affected(&mut self, mut op: impl FnMut(&mut AffectedTracker<'app>)) {
        if let Some(affected) = self.affected.as_mut() {
            op(affected);
        }
    }
}
