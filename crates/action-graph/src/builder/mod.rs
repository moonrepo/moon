//! A request-based action graph builder, which builds in stages with explicit
//! intermediate data, instead of inserting one node at a time. See
//! `docs/todo/ACTION_GRAPH_BUILDER.md` for the plan, and `docs/ACTION_GRAPH.md`
//! for the nodes and edges that it must produce.
//!
//! Requests are only recorded, and all of the work happens in [`ActionGraphBuilder::build`]:
//!
//! 1. Select the task instances and project syncs that exist, and how they
//!    relate, as an event log ([`select`]).
//! 2. Replay the events to assemble the nodes and edges, resolving each task's
//!    provisioning actions along the way, then 3. add the edges that depend on
//!    the whole graph, and 4. freeze it ([`assemble`]).

mod assemble;
mod select;

use crate::action_graph::ActionGraph;
use crate::action_graph_error::ActionGraphError;
use crate::action_graph_options::{ActionGraphBuilderOptions, RunPartition, RunRequirements};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, Depth};
use moon_app_context::AppContext;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_exec_plan::{ExecutionPlan, TargetsBlock};
use moon_project::Project;
use moon_query::build_query;
use moon_task::{Target, TargetLocator, Task};
use moon_toolchain::{ToolchainSpec, get_toolchain_spec};
use moon_workspace_graph::WorkspaceGraph;
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use select::{ProvisionRequest, RequestResult, TaskSelector};
use std::sync::Arc;
use tracing::debug;

/// Identifies a request, so that its result can be read from the [`BuildReport`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RequestId(usize);

#[derive(Debug)]
enum Request {
    Provision(ProvisionRequest),
    RunTasks(Vec<TargetLocator>),
    RunTasksWithPlan(TargetsBlock),
    RunTask(Arc<Task>),
    RunTaskByTarget(Target),
    RunTaskByTargetLocator(TargetLocator),
    #[cfg(debug_assertions)]
    RunTaskWithConfig(Arc<Task>, moon_config::TaskDependencyConfig),
    SyncProject(Arc<Project>),
}

/// The spec of a toolchain for a project, which may override its version.
pub(crate) fn get_spec(
    app_context: &AppContext,
    toolchain_id: &Id,
    project: Option<&Project>,
) -> Option<ToolchainSpec> {
    get_toolchain_spec(
        &app_context.toolchains_config,
        toolchain_id,
        project.map(|project| &project.config),
    )
}

/// The spec of a toolchain for a project, which may override its version, or
/// disable the toolchain.
pub(crate) fn get_project_spec(
    app_context: &AppContext,
    toolchain_id: &Id,
    project: &Project,
) -> Option<ToolchainSpec> {
    get_spec(app_context, toolchain_id, Some(project))
}

/// The spec of a configured toolchain.
pub(crate) fn get_workspace_spec(
    app_context: &AppContext,
    toolchain_id: &Id,
) -> Option<ToolchainSpec> {
    get_spec(app_context, toolchain_id, None)
}

/// The result of a build.
pub struct BuildOutput {
    pub context: ActionContext,
    pub graph: ActionGraph,
    pub report: BuildReport,
}

/// What each request produced, by the [`RequestId`] it returned.
#[derive(Debug, Default)]
pub struct BuildReport {
    nodes: FxHashMap<RequestId, Vec<NodeIndex>>,
    partitions: FxHashMap<RequestId, RunPartition>,
}

impl BuildReport {
    /// The primary nodes of a `run_tasks` request, and the partition size.
    pub fn partition(&self, id: RequestId) -> Option<&RunPartition> {
        self.partitions.get(&id)
    }

    /// Take ownership of the partition of a `run_tasks` request.
    pub fn take_partition(&mut self, id: RequestId) -> Option<RunPartition> {
        self.partitions.remove(&id)
    }

    /// The nodes that a request inserted, or that already existed for it.
    pub fn nodes(&self, id: RequestId) -> &[NodeIndex] {
        self.nodes
            .get(&id)
            .map(|nodes| nodes.as_slice())
            .unwrap_or(&[])
    }
}

pub struct ActionGraphBuilder {
    app_context: Arc<AppContext>,
    options: ActionGraphBuilderOptions,

    workspace_graph: Arc<WorkspaceGraph>,
    requirements: RunRequirements,
    requests: Vec<Request>,

    affected: Option<(Depth, bool)>,
    changed_files: Option<FxHashSet<WorkspaceRelativePathBuf>>,
    query: Option<String>,

    #[cfg(debug_assertions)]
    mocked_affected: Option<(AffectedTracker, Depth)>,
}

impl ActionGraphBuilder {
    pub fn new(
        app_context: Arc<AppContext>,
        workspace_graph: Arc<WorkspaceGraph>,
        options: ActionGraphBuilderOptions,
    ) -> Self {
        Self {
            app_context,
            options,
            workspace_graph,
            requirements: RunRequirements::default(),
            requests: vec![],
            affected: None,
            changed_files: None,
            query: None,
            #[cfg(debug_assertions)]
            mocked_affected: None,
        }
    }

    // SPECS

    /// The spec of a toolchain for the project, or the workspace.
    pub fn get_spec(&self, toolchain_id: &Id, project: Option<&Project>) -> Option<ToolchainSpec> {
        get_spec(&self.app_context, toolchain_id, project)
    }

    /// The spec of a toolchain for the project.
    pub fn get_project_spec(&self, toolchain_id: &Id, project: &Project) -> Option<ToolchainSpec> {
        get_project_spec(&self.app_context, toolchain_id, project)
    }

    /// The spec of a configured toolchain.
    pub fn get_workspace_spec(&self, toolchain_id: &Id) -> Option<ToolchainSpec> {
        get_workspace_spec(&self.app_context, toolchain_id)
    }

    // SETTINGS

    /// Set the requirements that every task request is built with.
    pub fn set_requirements(&mut self, requirements: RunRequirements) {
        self.requirements = requirements;
    }

    /// Set the changed files, which are required for affected tracking, and
    /// are passed through to the action context.
    pub fn set_changed_files(&mut self, changed_files: FxHashSet<WorkspaceRelativePathBuf>) {
        self.changed_files = Some(changed_files);
    }

    /// Bound `:task` targets, and the dependents of selected tasks, by a query.
    pub fn set_query(&mut self, input: &str) -> miette::Result<()> {
        // Validate it now, so that an invalid query errors where it was set
        build_query(input)?;

        self.query = Some(input.to_owned());

        Ok(())
    }

    /// Only insert affected tasks (and projects), where affected state flows
    /// through relations up to the depth. Requires changed files.
    pub fn track_affected(&mut self, depth: Depth, ci_check: bool) {
        self.affected = Some((depth, ci_check));
    }

    // REQUESTS

    fn request(&mut self, request: Request) -> RequestId {
        let id = RequestId(self.requests.len());

        self.requests.push(request);

        id
    }

    /// Run the tasks of the provided locators as primary targets, along with
    /// their dependents (up to the depth), partitioned across jobs.
    pub fn run_tasks<I, T>(&mut self, locators: I) -> RequestId
    where
        I: IntoIterator<Item = T>,
        T: AsRef<TargetLocator>,
    {
        self.request(Request::RunTasks(
            locators
                .into_iter()
                .map(|locator| locator.as_ref().to_owned())
                .collect(),
        ))
    }

    /// Run the targets of an execution plan, like [`Self::run_tasks`].
    pub fn run_tasks_with_plan(&mut self, plan: &ExecutionPlan) -> RequestId {
        self.request(Request::RunTasksWithPlan(plan.targets.clone()))
    }

    /// Run the provided task as a primary target, and when requested, its
    /// dependents (up to the depth) as primary targets as well. Takes the
    /// task's `Arc` (or an owned task), so that it's not cloned.
    pub fn run_task(&mut self, task: impl Into<Arc<Task>>) -> RequestId {
        self.request(Request::RunTask(task.into()))
    }

    /// Run the tasks of the provided target (and their dependents).
    pub fn run_task_by_target<T: AsRef<Target>>(&mut self, target: T) -> RequestId {
        self.request(Request::RunTaskByTarget(target.as_ref().to_owned()))
    }

    /// Run the tasks of the provided locator (and their dependents).
    pub fn run_task_by_target_locator<T: AsRef<TargetLocator>>(&mut self, locator: T) -> RequestId {
        self.request(Request::RunTaskByTargetLocator(locator.as_ref().to_owned()))
    }

    /// Install the dependencies of the toolchain for the project, in the
    /// closest root (or set it up, when it doesn't install dependencies).
    pub fn install_dependencies(
        &mut self,
        spec: &ToolchainSpec,
        project: impl Into<Arc<Project>>,
    ) -> RequestId {
        self.request(Request::Provision(ProvisionRequest::InstallDependencies(
            spec.to_owned(),
            project.into(),
        )))
    }

    /// Install the dependencies of each of the project's toolchains (or set
    /// them up, when they don't install dependencies).
    pub fn install_dependencies_by_project(
        &mut self,
        project: impl Into<Arc<Project>>,
    ) -> RequestId {
        self.request(Request::Provision(
            ProvisionRequest::InstallDependenciesByProject(project.into()),
        ))
    }

    /// Install dependencies in the workspace root, without an associated
    /// project, when the located dependencies root is the workspace root.
    pub fn install_dependencies_root(&mut self, spec: &ToolchainSpec) -> RequestId {
        self.request(Request::Provision(
            ProvisionRequest::InstallDependenciesRoot(spec.to_owned()),
        ))
    }

    /// Setup the environment of the toolchain in the provided root, for the
    /// project.
    pub fn setup_environment(
        &mut self,
        spec: &ToolchainSpec,
        root: &WorkspaceRelativePathBuf,
        project: impl Into<Arc<Project>>,
    ) -> RequestId {
        self.request(Request::Provision(ProvisionRequest::SetupEnvironment(
            spec.to_owned(),
            root.to_owned(),
            project.into(),
        )))
    }

    /// Setup proto itself.
    pub fn setup_proto(&mut self) -> RequestId {
        self.request(Request::Provision(ProvisionRequest::SetupProto))
    }

    /// Setup the toolchain, for the project when provided.
    pub fn setup_toolchain(
        &mut self,
        spec: &ToolchainSpec,
        project: Option<&Project>,
    ) -> RequestId {
        self.request(Request::Provision(ProvisionRequest::SetupToolchain(
            spec.to_owned(),
            project.map(|project| Arc::new(project.to_owned())),
        )))
    }

    /// Setup the toolchain, and when it inserted a node, its environment in
    /// the workspace root (like `moon setup`). The report has the toolchain's
    /// node, when it was inserted.
    pub fn setup_toolchain_and_environment(&mut self, spec: &ToolchainSpec) -> RequestId {
        self.request(Request::Provision(
            ProvisionRequest::SetupToolchainAndEnvironment(spec.to_owned()),
        ))
    }

    /// Setup the environment in the workspace root, without an associated
    /// project, when the located dependencies root is the workspace root.
    pub fn setup_environment_root(&mut self, spec: &ToolchainSpec) -> RequestId {
        self.request(Request::Provision(ProvisionRequest::SetupEnvironmentRoot(
            spec.to_owned(),
        )))
    }

    /// Sync the project, and when enabled, the projects it depends on. Takes
    /// the project's `Arc` (or an owned project), so that it's not cloned.
    pub fn sync_project(&mut self, project: impl Into<Arc<Project>>) -> RequestId {
        self.request(Request::SyncProject(project.into()))
    }

    /// Sync the workspace.
    pub fn sync_workspace(&mut self) -> RequestId {
        self.request(Request::Provision(ProvisionRequest::SyncWorkspace))
    }

    // BUILD

    /// Build the graph from every request, in the order they were made.
    pub async fn build(mut self) -> miette::Result<BuildOutput> {
        debug!("Building action graph");

        // Validate the job up front, before any work is done
        if let (Some(index), Some(total)) = (self.requirements.job, self.requirements.job_total)
            && index >= total
        {
            return Err(ActionGraphError::InvalidJobIndex { index, total }.into());
        }

        let changed_files = self.changed_files.take();
        let (affected, affected_depth) = self.create_affected_tracker(&changed_files).await?;

        let query = match &self.query {
            Some(input) => Some(build_query(input)?),
            None => None,
        };

        let mut selector = TaskSelector::new(
            Arc::clone(&self.workspace_graph),
            query.as_ref(),
            affected,
            affected_depth,
            self.options.sync_projects.clone(),
            self.options.sync_project_dependencies,
        );
        let reqs = self.requirements.clone();

        for (index, request) in std::mem::take(&mut self.requests).into_iter().enumerate() {
            let id = RequestId(index);

            match request {
                Request::Provision(provision) => selector.provision(id, provision),
                Request::SyncProject(project) => selector.sync_project(id, &project, &reqs)?,
                Request::RunTasks(locators) => selector.run_tasks(id, &locators, &reqs).await?,
                Request::RunTasksWithPlan(targets) => {
                    selector.run_tasks_with_plan(id, &targets, &reqs).await?
                }
                Request::RunTask(task) => selector.run_task(id, &task, &reqs)?,
                Request::RunTaskByTarget(target) => {
                    selector.run_task_by_target(id, &target, &reqs)?
                }
                Request::RunTaskByTargetLocator(locator) => {
                    selector.run_task_by_target_locator(id, &locator, &reqs)?
                }
                #[cfg(debug_assertions)]
                Request::RunTaskWithConfig(task, config) => {
                    selector.run_task_with_config(id, &task, &config, &reqs)?
                }
            };
        }

        let mut plan = selector.finish();
        let assembled = assemble::assemble(
            &self.app_context,
            &self.workspace_graph,
            &self.options,
            &mut plan,
        )
        .await?;

        let mut report = BuildReport {
            nodes: assembled.request_nodes,
            ..BuildReport::default()
        };

        for (id, result) in plan.results {
            match result {
                RequestResult::Partition { targets, size } => {
                    report.partitions.insert(
                        id,
                        RunPartition {
                            targets: targets
                                .into_iter()
                                .map(|(instance, target)| {
                                    (assembled.instance_nodes[instance], target)
                                })
                                .collect(),
                            size,
                        },
                    );
                }
                RequestResult::Instances(instances) => {
                    report.nodes.insert(
                        id,
                        instances
                            .into_iter()
                            .map(|instance| assembled.instance_nodes[instance])
                            .collect(),
                    );
                }
                RequestResult::SyncProject(project_id) => {
                    report.nodes.insert(
                        id,
                        project_id
                            .and_then(|id| assembled.sync_project_nodes.get(&id).copied())
                            .into_iter()
                            .collect(),
                    );
                }
            };
        }

        let mut context = ActionContext {
            affected: plan.affected.map(|affected| affected.build()),
            cleanup_targets: plan.cleanup_targets,
            ignored_dependencies: assembled.ignored_dependencies,
            primary_targets: plan.primary_targets,
            wait_targets: plan.wait_targets,
            ..ActionContext::default()
        };

        context.wait_targets.extend(assembled.wait_targets);

        for target in plan.passthrough_targets {
            context.set_target_state(target, TargetState::Passthrough);
        }

        if let Some(files) = changed_files {
            context.changed_files = files;
        }

        Ok(BuildOutput {
            context,
            graph: assembled.graph,
            report,
        })
    }

    async fn create_affected_tracker(
        &mut self,
        changed_files: &Option<FxHashSet<WorkspaceRelativePathBuf>>,
    ) -> miette::Result<(Option<AffectedTracker>, Depth)> {
        #[cfg(debug_assertions)]
        if let Some((affected, depth)) = self.mocked_affected.take() {
            return Ok((Some(affected), depth));
        }

        let Some((depth, ci_check)) = self.affected else {
            return Ok((None, Depth::Deep));
        };

        // If affected state flows through relations, then we must load all
        // projects into the graph so that the edges are created!
        if !depth.is_none() {
            debug!("Force loading all projects and tasks to determine relationships");

            self.workspace_graph.get_projects()?;
            self.workspace_graph.get_tasks_with_internal()?;
        }

        let Some(changed_files) = changed_files.clone() else {
            return Err(ActionGraphError::MissingChangedFiles.into());
        };

        let mut affected = AffectedTracker::new(Arc::clone(&self.workspace_graph), changed_files);

        affected.set_ci_check(ci_check);
        affected.set_depth(depth);

        // Projects must be tracked up front so that all marks exist
        // before tasks are selected, regardless of their order
        affected.track_projects().await?;

        Ok((Some(affected), depth))
    }
}

#[cfg(debug_assertions)]
impl ActionGraphBuilder {
    /// Track affected state for the changed files, without tracking projects
    /// up front, so that tasks and projects are only checked when inserted.
    pub fn set_affected(&mut self) -> miette::Result<()> {
        let Some(changed_files) = self.changed_files.clone() else {
            return Err(ActionGraphError::MissingChangedFiles.into());
        };

        self.mocked_affected = Some((
            AffectedTracker::new(Arc::clone(&self.workspace_graph), changed_files),
            Depth::Deep,
        ));

        Ok(())
    }

    /// Run the provided task with the args and env of a dependency config, but
    /// not as a primary target, and without its dependents.
    pub fn run_task_with_config(
        &mut self,
        task: &Task,
        config: &moon_config::TaskDependencyConfig,
    ) -> RequestId {
        self.request(Request::RunTaskWithConfig(
            Arc::new(task.to_owned()),
            config.to_owned(),
        ))
    }

    /// Track affected state with a tracker that the test marks itself.
    pub fn mock_affected(
        &mut self,
        changed_files: FxHashSet<WorkspaceRelativePathBuf>,
        depth: Depth,
        mut op: impl FnMut(&mut AffectedTracker),
    ) {
        let mut affected =
            AffectedTracker::new(Arc::clone(&self.workspace_graph), changed_files.clone());

        affected.set_depth(depth);

        op(&mut affected);

        self.changed_files = Some(changed_files);
        self.mocked_affected = Some((affected, depth));
    }
}
