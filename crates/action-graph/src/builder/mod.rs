//! A request-based action graph builder, which builds in stages with explicit
//! intermediate data, instead of inserting one node at a time. See
//! `docs/todo/ACTION_GRAPH_BUILDER.md` for the plan, and `docs/ACTION_GRAPH.md`
//! for the nodes and edges that it must produce.
//!
//! Requests are only recorded, and all of the work happens in [`ActionGraphBuilder::build`]:
//!
//! 1. Select the task instances that exist, and how they relate ([`select`]).
//! 2. Resolve the provisioning and sync actions (not implemented yet).
//! 3. Assemble the nodes and edges, then 4. order and validate ([`assemble`]).
//!
//! Until provisioning is implemented, this builder produces the task subgraph
//! only: the graph that [`crate::ActionGraphBuilder`] produces when every
//! builder option is disabled.

mod assemble;
mod select;

use crate::action_graph::ActionGraph;
use crate::action_graph_builder::{ActionGraphBuilderOptions, RunPartition, RunRequirements};
use moon_action_context::{ActionContext, TargetState};
use moon_affected::{AffectedTracker, Depth};
use moon_app_context::AppContext;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_exec_plan::{ExecutionPlan, TargetsBlock};
use moon_query::build_query;
use moon_task::{Target, TargetLocator, Task};
use moon_workspace_graph::WorkspaceGraph;
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use select::{RequestResult, TaskSelector};
use std::sync::Arc;
use tracing::debug;

/// Identifies a request, so that its result can be read from the [`BuildReport`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RequestId(usize);

// Provisioning and sync requests, without the prefix, are added next
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
enum Request {
    RunTasks(Vec<TargetLocator>),
    RunTasksWithPlan(TargetsBlock),
    RunTask(Arc<Task>),
    RunTaskByTarget(Target),
    RunTaskByTargetLocator(TargetLocator),
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

    /// The nodes that a request inserted, or that already existed for it.
    pub fn nodes(&self, id: RequestId) -> &[NodeIndex] {
        self.nodes
            .get(&id)
            .map(|nodes| nodes.as_slice())
            .unwrap_or(&[])
    }
}

pub struct ActionGraphBuilder {
    // Read once provisioning is implemented
    #[allow(dead_code)]
    app_context: Arc<AppContext>,
    #[allow(dead_code)]
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
    /// dependents (up to the depth) as primary targets as well.
    pub fn run_task(&mut self, task: &Task) -> RequestId {
        self.request(Request::RunTask(Arc::new(task.to_owned())))
    }

    /// Run the tasks of the provided target (and their dependents).
    pub fn run_task_by_target<T: AsRef<Target>>(&mut self, target: T) -> RequestId {
        self.request(Request::RunTaskByTarget(target.as_ref().to_owned()))
    }

    /// Run the tasks of the provided locator (and their dependents).
    pub fn run_task_by_target_locator<T: AsRef<TargetLocator>>(&mut self, locator: T) -> RequestId {
        self.request(Request::RunTaskByTargetLocator(locator.as_ref().to_owned()))
    }

    // BUILD

    /// Build the graph from every request, in the order they were made.
    pub async fn build(mut self) -> miette::Result<BuildOutput> {
        debug!("Building action graph");

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
        );
        let reqs = self.requirements.clone();

        for (index, request) in self.requests.iter().enumerate() {
            let id = RequestId(index);

            match request {
                Request::RunTasks(locators) => selector.run_tasks(id, locators, &reqs).await?,
                Request::RunTasksWithPlan(targets) => {
                    selector.run_tasks_with_plan(id, targets, &reqs).await?
                }
                Request::RunTask(task) => selector.run_task(id, task, &reqs)?,
                Request::RunTaskByTarget(target) => {
                    selector.run_task_by_target(id, target, &reqs)?
                }
                Request::RunTaskByTargetLocator(locator) => {
                    selector.run_task_by_target_locator(id, locator, &reqs)?
                }
            };
        }

        let mut plan = selector.finish();
        let assembled = assemble::assemble(&mut plan)?;

        // Node indices are the indices of the task instances
        let mut report = BuildReport::default();

        for (id, result) in plan.results {
            match result {
                RequestResult::Partition { targets, size } => {
                    report.partitions.insert(
                        id,
                        RunPartition {
                            targets: targets
                                .into_iter()
                                .map(|(index, target)| (NodeIndex::new(index), target))
                                .collect(),
                            size,
                        },
                    );
                }
                RequestResult::Instances(indices) => {
                    report
                        .nodes
                        .insert(id, indices.into_iter().map(NodeIndex::new).collect());
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

        let mut affected = AffectedTracker::new(
            Arc::clone(&self.workspace_graph),
            changed_files
                .clone()
                .expect("Changed files are required for affected tracking."),
        );

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
