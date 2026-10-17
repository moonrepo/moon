//! Stage 1: select the task instances and project syncs that exist, and how
//! they relate.
//!
//! This doesn't create any graph nodes or edges. It walks the requests, the
//! dependencies of each task they reach, and the dependencies of each project
//! that's synced, applying the same gates in the same order as the action graph
//! has always done (affected checks have side effects, so their order matters),
//! and records the result as data: an event log in the order that the nodes are
//! inserted and the edges are linked, and the serial ordering links.

use super::RequestId;
use crate::action_graph_error::ActionGraphError;
use crate::action_graph_options::{RunRequirements, TaskOrigin};
use moon_action::{ActionNode, RunTaskNode};
use moon_affected::{AffectedTracker, Depth};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, color};
use moon_config::{EnvMap, PipelineActionSwitch, TaskDependencyConfig, TaskDependencyType};
use moon_exec_plan::TargetsBlock;
use moon_project::{Project, ProjectError};
use moon_query::{Criteria, build_query};
use moon_task::{Target, TargetError, TargetLocator, TargetProjectScope, TargetTaskScope, Task};
use moon_toolchain::ToolchainSpec;
use moon_workspace_graph::projects::ProjectGraphError;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use petgraph::Direction;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use tracing::debug;

/// The identity of a task instance: its target, and the args and env of the
/// dependency that inserted it. The interactive and persistent flags aren't
/// part of it, as there's one set of requirements per build, so they're a
/// function of the target.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TaskInstanceKey {
    target: Target,
    args: Vec<String>,
    env: Vec<(String, Option<String>)>,
}

impl TaskInstanceKey {
    fn new(target: &Target, args: &[String], env: &EnvMap) -> Self {
        let mut env = env
            .iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect::<Vec<_>>();

        // Maps are equal regardless of the order of their keys
        env.sort_by(|a, b| a.0.cmp(&b.0));

        Self {
            target: target.to_owned(),
            args: args.to_vec(),
            env,
        }
    }
}

/// A request for a provisioning or sync action, which is resolved while
/// assembling, in the order the requests were made.
#[derive(Debug)]
pub enum ProvisionRequest {
    InstallDependencies(ToolchainSpec, Arc<Project>),
    InstallDependenciesByProject(Arc<Project>),
    InstallDependenciesRoot(ToolchainSpec),
    SetupEnvironment(ToolchainSpec, WorkspaceRelativePathBuf, Arc<Project>),
    SetupProto,
    SetupToolchain(ToolchainSpec, Option<Arc<Project>>),
    /// Set up the toolchain, and when it inserted a node, the environment in
    /// the workspace root (like `moon setup`).
    SetupToolchainAndEnvironment(ToolchainSpec),
    SetupEnvironmentRoot(ToolchainSpec),
    SyncWorkspace,
}

/// What happened, in the order that the nodes are inserted, and the edges
/// are linked, so that assembling produces the same node indices as always.
#[derive(Debug)]
pub enum Event {
    /// A call to sync the workspace, which inserts its node on the first call.
    SyncWorkspace,
    /// A project sync that was inserted.
    AddSyncProject(Id),
    /// Link a project sync to the workspace sync, once the syncs of its
    /// dependencies have been inserted. The edges to those syncs are linked
    /// once every project sync is known.
    LinkSyncProject(Id),
    /// A task instance that was inserted, after its owning project's sync (when
    /// it's synced), and before its dependencies. Its toolchains are set up
    /// (and their dependencies installed) before its node is inserted.
    EnterTask {
        instance: usize,
        project: Arc<Project>,
        task: Arc<Task>,
        sync_project: Option<Id>,
    },
    /// A task instance whose dependencies have been visited: link its setup
    /// and sync edges, then its requirements, then its cleanups (reversed).
    ExitTask {
        instance: usize,
        requirements: Vec<(usize, TaskDependencyType)>,
        cleanups: Vec<usize>,
    },
    /// A provisioning or sync request.
    Request(RequestId, ProvisionRequest),
}

/// A serial ordering link: the dependency at `index` (and its subtree) must
/// run after the previous dependency of the same parent.
#[derive(Debug)]
pub struct SerialLink {
    pub index: usize,
    pub previous: Option<usize>,
    pub previous_standard: Option<usize>,
}

/// What a request produced.
#[derive(Debug)]
pub enum RequestResult {
    /// The primary instances of a `run_tasks` request, and the partition size.
    Partition {
        targets: Vec<(usize, Target)>,
        size: Option<usize>,
    },
    /// The primary instances of any other task request.
    Instances(Vec<usize>),
    /// The project of a project sync request, when it passed the gates.
    SyncProject(Option<Id>),
}

/// Everything selected for the task requests of a build.
pub struct TaskPlan {
    /// The `RunTask` payload of each instance, in discovery order.
    pub instances: Vec<ActionNode>,
    /// The events, in the order that nodes are inserted, and edges are linked.
    pub events: Vec<Event>,
    /// The projects that are synced, as their syncs depend on each other.
    pub synced_projects: Vec<Arc<Project>>,
    /// Serial ordering links, in the order that they're made.
    pub serial_links: Vec<SerialLink>,
    /// The dependencies (that aren't a cleanup) of tasks that were inserted
    /// without them (`no_dependencies`), linked to the instances that exist.
    pub no_dependencies_tasks: FxHashMap<Target, Vec<TaskDependencyConfig>>,
    pub affected: Option<AffectedTracker>,
    pub cleanup_targets: FxHashSet<Target>,
    pub passthrough_targets: FxHashSet<Target>,
    pub primary_targets: FxHashSet<Target>,
    pub wait_targets: FxHashSet<Target>,
    pub results: FxHashMap<RequestId, RequestResult>,
}

/// The tasks that a dependency target resolves to, shared by every task
/// that depends on it.
type DependencyTasks = Arc<[Arc<Task>]>;

/// A task whose dependencies are being visited.
struct Frame {
    instance: usize,
    parallel: bool,
    task: Arc<Task>,
    /// Only these dependencies of the task are visited, instead of all of them.
    only_deps: Option<Vec<TaskDependencyConfig>>,
    next_dep: usize,
    /// The dependency being visited, its resolved tasks, and the next one.
    current: Option<(usize, DependencyTasks, usize)>,
    /// A dependency instance that was just created, and is linked once its own
    /// dependencies have been visited (when its frame is popped).
    pending: Option<(usize, Arc<Task>, usize)>,
    previous: Option<usize>,
    previous_standard: Option<usize>,
    requirements: Vec<(usize, TaskDependencyType)>,
    cleanups: Vec<usize>,
}

impl Frame {
    fn deps(&self) -> &[TaskDependencyConfig] {
        self.only_deps.as_deref().unwrap_or(&self.task.deps)
    }
}

enum Entry {
    /// The task didn't pass a gate.
    Skipped,
    /// An instance with the same identity already exists.
    Existing(usize),
    /// A new instance, and a frame when it has dependencies to visit.
    New(usize, Option<Frame>),
}

/// A project sync whose dependencies are being visited.
struct SyncFrame {
    project: Arc<Project>,
    deps: Vec<Id>,
    next_dep: usize,
}

enum SyncEntry {
    Skipped,
    Existing,
    New(SyncFrame),
}

pub struct TaskSelector<'query> {
    workspace_graph: Arc<WorkspaceGraph>,
    query: Option<&'query Criteria<'query>>,
    affected: Option<AffectedTracker>,
    affected_depth: Depth,
    sync_projects: PipelineActionSwitch,
    sync_project_dependencies: bool,

    events: Vec<Event>,
    instances: Vec<ActionNode>,
    instance_indices: FxHashMap<TaskInstanceKey, usize>,
    synced_projects: Vec<Arc<Project>>,
    synced_project_ids: FxHashSet<Id>,
    serial_links: Vec<SerialLink>,
    no_dependencies_tasks: FxHashMap<Target, Vec<TaskDependencyConfig>>,
    cleanup_targets: FxHashSet<Target>,
    passthrough_targets: FxHashSet<Target>,
    primary_targets: FxHashSet<Target>,
    wait_targets: FxHashSet<Target>,
    results: FxHashMap<RequestId, RequestResult>,

    /// The tasks of each dependency target, which are resolved once.
    dependency_tasks: FxHashMap<Target, DependencyTasks>,
}

impl<'query> TaskSelector<'query> {
    pub fn new(
        workspace_graph: Arc<WorkspaceGraph>,
        query: Option<&'query Criteria<'query>>,
        affected: Option<AffectedTracker>,
        affected_depth: Depth,
        sync_projects: PipelineActionSwitch,
        sync_project_dependencies: bool,
    ) -> Self {
        Self {
            workspace_graph,
            query,
            affected,
            affected_depth,
            sync_projects,
            sync_project_dependencies,
            events: vec![],
            instances: vec![],
            instance_indices: FxHashMap::default(),
            synced_projects: vec![],
            synced_project_ids: FxHashSet::default(),
            serial_links: vec![],
            no_dependencies_tasks: FxHashMap::default(),
            cleanup_targets: FxHashSet::default(),
            passthrough_targets: FxHashSet::default(),
            primary_targets: FxHashSet::default(),
            wait_targets: FxHashSet::default(),
            results: FxHashMap::default(),
            dependency_tasks: FxHashMap::default(),
        }
    }

    pub fn finish(self) -> TaskPlan {
        TaskPlan {
            instances: self.instances,
            events: self.events,
            synced_projects: self.synced_projects,
            serial_links: self.serial_links,
            no_dependencies_tasks: self.no_dependencies_tasks,
            affected: self.affected,
            cleanup_targets: self.cleanup_targets,
            passthrough_targets: self.passthrough_targets,
            primary_targets: self.primary_targets,
            wait_targets: self.wait_targets,
            results: self.results,
        }
    }

    // REQUESTS

    /// Record a provisioning or sync request, which is resolved in order.
    pub fn provision(&mut self, id: RequestId, request: ProvisionRequest) {
        self.events.push(Event::Request(id, request));
    }

    /// Sync the project, and when enabled, the projects it depends on.
    pub fn sync_project(
        &mut self,
        id: RequestId,
        project: &Arc<Project>,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let synced = self.walk_sync_project(project, reqs.skip_affected)?;

        self.results.insert(
            id,
            RequestResult::SyncProject(synced.then(|| project.id.clone())),
        );

        Ok(())
    }

    /// Run the tasks of the provided locators as primary targets, along with
    /// their dependents (up to the depth), partitioned across jobs.
    pub async fn run_tasks(
        &mut self,
        id: RequestId,
        locators: &[TargetLocator],
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let (targets, size) = self.select_tasks(locators, reqs).await?;

        self.results
            .insert(id, RequestResult::Partition { targets, size });

        Ok(())
    }

    /// Run the targets of an execution plan, like [`Self::run_tasks`].
    pub async fn run_tasks_with_plan(
        &mut self,
        id: RequestId,
        targets: &TargetsBlock,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let (targets, size) = match targets {
            TargetsBlock::Partitioned { jobs } => {
                let (Some(job_index), Some(job_total)) = (reqs.job, reqs.job_total) else {
                    return Err(ActionGraphError::InvalidPlanJobs.into());
                };

                if job_total == 0 {
                    return Err(ActionGraphError::InvalidPlanJobs.into());
                }

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

                // The plan already partitioned the targets
                let reqs = RunRequirements {
                    job: None,
                    job_total: None,
                    ..reqs.clone()
                };
                let targets = &jobs[job_index];

                (
                    self.select_tasks(targets, &reqs).await?.0,
                    Some(targets.len()),
                )
            }
            TargetsBlock::Filtered { include } => self.select_tasks(include, reqs).await?,
            TargetsBlock::Included(targets) => self.select_tasks(targets, reqs).await?,
        };

        self.results
            .insert(id, RequestResult::Partition { targets, size });

        Ok(())
    }

    /// Run the provided task as a primary target, and when requested, its
    /// dependents (up to the depth) as primary targets as well.
    pub fn run_task(
        &mut self,
        id: RequestId,
        task: &Arc<Task>,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let mut indices = vec![];

        if let Some(index) = self.insert_primary(task, reqs, TaskOrigin::Requested)? {
            indices.push(index);
        }

        for dep_task in self.resolve_dependents(&[task.as_ref()], reqs.dependents)? {
            self.insert_primary(&dep_task, reqs, TaskOrigin::Dependent)?;
        }

        self.results.insert(id, RequestResult::Instances(indices));

        Ok(())
    }

    /// Run the provided task with the args and env of a dependency config, but
    /// not as a primary target, and without its dependents.
    #[cfg(debug_assertions)]
    pub fn run_task_with_config(
        &mut self,
        id: RequestId,
        task: &Arc<Task>,
        config: &TaskDependencyConfig,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let indices = self
            .insert(task, Some(config), reqs, TaskOrigin::Requested)?
            .into_iter()
            .collect();

        self.results.insert(id, RequestResult::Instances(indices));

        Ok(())
    }

    /// Run the tasks of the provided target (and their dependents).
    pub fn run_task_by_target(
        &mut self,
        id: RequestId,
        target: &Target,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let tasks = self.resolve_tasks_from_target(target, TaskOrigin::Requested)?;

        self.run_primary_tasks(id, tasks, reqs)
    }

    /// Run the tasks of the provided locator (and their dependents).
    pub fn run_task_by_target_locator(
        &mut self,
        id: RequestId,
        locator: &TargetLocator,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let tasks = self.resolve_tasks_from_target_locator(locator)?;

        self.run_primary_tasks(id, tasks, reqs)
    }

    // SELECTION

    async fn select_tasks(
        &mut self,
        locators: &[TargetLocator],
        reqs: &RunRequirements,
    ) -> miette::Result<(Vec<(usize, Target)>, Option<usize>)> {
        let mut tasks = vec![];

        for locator in locators {
            tasks.extend(self.resolve_tasks_from_target_locator(locator)?);
        }

        // Expand the selection with the dependents of the selected tasks up
        // front, so that the candidates don't depend on which tasks have been
        // inserted (or skipped) by the time the dependents are expanded
        let mut candidates = self.collect_candidates(tasks, reqs.dependents)?;

        // Determine the affected state of each task up front, as marking
        // lazily while inserting would depend on the order of the targets.
        // Tasks are tracked even when affected checks are skipped (`--force`),
        // so that their affected state is still reported
        let affected_depth = self.affected_depth;

        if let Some(affected) = &mut self.affected {
            // When affected state flows through relations, every task must be
            // tracked, as a task is only marked through a relation when the
            // task on the other side of it has been marked itself
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

        let mut size = None;

        // Partition the candidates based on the job information, which
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

            // Filter based on affected state before partitioning
            if !reqs.skip_affected {
                let mut affected_candidates = vec![];

                for (task, origin) in candidates {
                    if self.is_task_affected(&task, reqs.skip_affected)? {
                        affected_candidates.push((task, origin));
                    }
                }

                candidates = affected_candidates;
            }

            let job_size = candidates.len().div_ceil(job_total);

            // The last jobs can start past the end (10 candidates over 6 jobs
            // leaves the last one empty), and must then run nothing, instead of
            // every candidate
            let stop = if job_index == job_total - 1 {
                candidates.len()
            } else {
                (job_size * (job_index + 1)).min(candidates.len())
            };
            let start = (job_size * job_index).min(stop);

            candidates.truncate(stop);
            candidates.drain(..start);

            size = Some(job_size);
        }

        let mut targets = vec![];

        for (task, origin) in candidates {
            if let Some(index) = self.insert_primary(&task, reqs, origin)? {
                targets.push((index, task.target.clone()));
            }
        }

        Ok((targets, size))
    }

    fn run_primary_tasks(
        &mut self,
        id: RequestId,
        tasks: Vec<Arc<Task>>,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let mut indices = vec![];

        for (task, origin) in self.collect_candidates(tasks, reqs.dependents)? {
            if let Some(index) = self.insert_primary(&task, reqs, origin)?
                && !indices.contains(&index)
            {
                indices.push(index);
            }
        }

        self.results.insert(id, RequestResult::Instances(indices));

        Ok(())
    }

    /// Dedupe the selected tasks, and append their dependents (up to the depth).
    /// A task that was both selected and reached as a dependent is requested.
    fn collect_candidates(
        &self,
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
                .map(|(task, _)| task.as_ref())
                .collect::<Vec<_>>();
            let dep_tasks = self.resolve_dependents(&seeds, dependents)?;

            for dep_task in dep_tasks {
                if seen.insert(dep_task.target.clone()) {
                    candidates.push((dep_task, TaskOrigin::Dependent));
                }
            }
        }

        Ok(candidates)
    }

    // INSERTION

    fn insert_primary(
        &mut self,
        task: &Arc<Task>,
        reqs: &RunRequirements,
        origin: TaskOrigin,
    ) -> miette::Result<Option<usize>> {
        let index = self.insert(task, None, reqs, origin)?;

        // Only primary targets that were inserted are tracked
        if index.is_some() {
            self.primary_targets.insert(task.target.clone());
        }

        Ok(index)
    }

    /// Insert the task, and its dependencies, depth first. The dependencies are
    /// walked with an explicit stack, instead of recursion, so that deep chains
    /// don't overflow the stack.
    fn insert(
        &mut self,
        task: &Arc<Task>,
        config: Option<&TaskDependencyConfig>,
        reqs: &RunRequirements,
        origin: TaskOrigin,
    ) -> miette::Result<Option<usize>> {
        let (root, frame) = match self.enter(task, config, origin, reqs.skip_affected, reqs)? {
            Entry::Skipped => return Ok(None),
            Entry::Existing(index) => return Ok(Some(index)),
            Entry::New(index, frame) => (index, frame),
        };

        // Without dependencies to visit, its edges are linked right away
        if frame.is_none() {
            self.exit_task(root, vec![], vec![]);
        }

        let mut stack = frame.into_iter().collect::<Vec<_>>();

        while let Some(frame) = stack.last_mut() {
            let next = loop {
                if let Some((dep_index, dep_tasks, next_task)) = &mut frame.current {
                    if let Some(dep_task) = dep_tasks.get(*next_task) {
                        *next_task += 1;

                        break Some((*dep_index, Arc::clone(dep_task)));
                    }

                    frame.current = None;
                }

                let Some(dep) = frame.deps().get(frame.next_dep) else {
                    break None;
                };

                let dep_tasks = self.resolve_dependency_tasks(&dep.target)?;

                frame.current = Some((frame.next_dep, dep_tasks, 0));
                frame.next_dep += 1;
            };

            let Some((dep_index, dep_task)) = next else {
                let frame = stack.pop().unwrap();

                self.link_dependencies(frame);

                if let Some(parent) = stack.last_mut()
                    && let Some((dep_index, dep_task, instance)) = parent.pending.take()
                {
                    self.link_dependency(parent, dep_index, &dep_task, instance);
                }

                continue;
            };

            // Dependencies are inserted regardless of affected state
            match self.enter(
                &dep_task,
                Some(&frame.deps()[dep_index]),
                TaskOrigin::Dependency,
                true,
                reqs,
            )? {
                Entry::Skipped => {}
                Entry::Existing(instance) => {
                    self.link_dependency(frame, dep_index, &dep_task, instance);
                }
                Entry::New(instance, None) => {
                    self.exit_task(instance, vec![], vec![]);
                    self.link_dependency(frame, dep_index, &dep_task, instance);
                }
                Entry::New(instance, Some(child)) => {
                    frame.pending = Some((dep_index, dep_task, instance));
                    stack.push(child);
                }
            };
        }

        Ok(Some(root))
    }

    /// Apply the gates to the task, and create an instance for it if it passes
    /// them, and an instance with the same identity doesn't exist yet.
    fn enter(
        &mut self,
        task: &Arc<Task>,
        config: Option<&TaskDependencyConfig>,
        origin: TaskOrigin,
        skip_affected: bool,
        reqs: &RunRequirements,
    ) -> miette::Result<Entry> {
        let project = self
            .workspace_graph
            .get_project(task.target.get_project_id()?)?;

        // Abort early if not affected. Dependencies are inserted regardless of
        // affected state, so they're not checked (nor marked)
        if origin != TaskOrigin::Dependency && !self.is_task_affected(task, skip_affected)? {
            return Ok(Entry::Skipped);
        }

        // These tasks shouldn't actually run, so filter them out
        if self.passthrough_targets.contains(&task.target) {
            debug!(
                task_target = task.target.as_str(),
                "Not running task {} because it has been marked as passthrough",
                color::id(&task.target.id),
            );

            return Ok(Entry::Skipped);
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

            return Ok(Entry::Skipped);
        }

        // Persistent tasks never complete, so they would keep the pipeline
        // running. When the targets were not requested (the command chose
        // them), they're only ran when another task depends on them. In CI,
        // they're also never ran as a dependent of another task
        if task.is_persistent()
            && origin != TaskOrigin::Dependency
            && (reqs.skip_persistent
                || (reqs.ci && reqs.ci_check && origin == TaskOrigin::Dependent))
        {
            debug!(
                task_target = task.target.as_str(),
                "Not running persistent task {} because it has not been requested, and no task depends on it",
                color::id(&task.target.id),
            );

            return Ok(Entry::Skipped);
        }

        let mut args = vec![];
        let mut env = EnvMap::default();

        if let Some(config) = config {
            args.extend(config.args.clone());
            env.extend(config.env.clone());
        }

        let key = TaskInstanceKey::new(&task.target, &args, &env);

        if let Some(index) = self.instance_indices.get(&key) {
            return Ok(Entry::Existing(*index));
        }

        // The owning project is synced before the task is inserted
        let sync_project = self
            .walk_sync_project(&project, skip_affected)?
            .then(|| project.id.clone());

        let index = self.instances.len();
        let node = ActionNode::run_task(RunTaskNode {
            args,
            env,
            interactive: task.is_interactive() || reqs.interactive,
            persistent: task.is_persistent(),
            priority: task.options.priority.get_level(),
            target: task.target.to_owned(),
            id: None,
        });

        debug!(
            index,
            "Adding {} to graph",
            color::muted_light(node.label())
        );

        self.instances.push(node);
        self.instance_indices.insert(key, index);
        self.events.push(Event::EnterTask {
            instance: index,
            project,
            task: Arc::clone(task),
            sync_project,
        });

        if task.deps.is_empty() {
            return Ok(Entry::New(index, None));
        }

        let only_deps = if reqs.no_dependencies {
            // Dependencies that weren't selected are not inserted, but the
            // ones that exist are still linked once every request has been
            // selected. Cleanup dependencies must always run after the task,
            // so they're still inserted and linked here
            let (cleanups, others) = task
                .deps
                .iter()
                .cloned()
                .partition(|dep| matches!(dep.type_of, TaskDependencyType::Cleanup));

            self.no_dependencies_tasks
                .insert(task.target.clone(), others);

            Some(cleanups)
        } else {
            None
        };

        Ok(Entry::New(
            index,
            Some(Frame {
                instance: index,
                parallel: task.options.run_deps_in_parallel,
                task: Arc::clone(task),
                only_deps,
                next_dep: 0,
                current: None,
                pending: None,
                previous: None,
                previous_standard: None,
                requirements: vec![],
                cleanups: vec![],
            }),
        ))
    }

    /// Record the relation between a task and one of its dependency instances.
    fn link_dependency(
        &mut self,
        frame: &mut Frame,
        dep_index: usize,
        dep_task: &Task,
        instance: usize,
    ) {
        let type_of = frame.deps()[dep_index].type_of;

        // Cleanup dependencies run *after* the task, so the relation is
        // reversed, and they take no part in the serial chain
        if matches!(type_of, TaskDependencyType::Cleanup) {
            self.cleanup_targets.insert(dep_task.target.clone());
            frame.cleanups.push(instance);

            return;
        }

        if matches!(type_of, TaskDependencyType::Wait) {
            self.wait_targets.insert(dep_task.target.clone());
        }

        // When serial, this dependency's entire subtree must run after the
        // previous dependency. Only dependencies that block the task are
        // chained, as a `wait` dependency doesn't run to completion first
        let is_required = type_of.is_required_type();

        if !frame.parallel && is_required {
            self.serial_links.push(SerialLink {
                index: instance,
                previous: frame.previous,
                previous_standard: frame.previous_standard,
            });
        }

        // The optional type is only a marker within the task
        // graph, the action graph has always used required
        frame.requirements.push((
            instance,
            if is_required {
                TaskDependencyType::Required
            } else {
                type_of
            },
        ));

        if is_required {
            frame.previous = Some(instance);

            // A persistent dependency never completes, so it can't order the
            // dependencies that come after it
            if !dep_task.is_persistent() {
                frame.previous_standard = Some(instance);
            }
        }
    }

    /// Record the relations of a task, once all of its dependencies have been
    /// visited: its requirements, then its cleanups (which are reversed).
    fn link_dependencies(&mut self, frame: Frame) {
        self.exit_task(frame.instance, frame.requirements, frame.cleanups);
    }

    fn exit_task(
        &mut self,
        instance: usize,
        requirements: Vec<(usize, TaskDependencyType)>,
        cleanups: Vec<usize>,
    ) {
        self.events.push(Event::ExitTask {
            instance,
            requirements,
            cleanups,
        });
    }

    // PROJECT SYNCS

    /// Sync the project, and when enabled, the projects it depends on, depth
    /// first. Returns whether the project is synced. Dependencies are walked
    /// with an explicit stack, so that deep chains don't overflow the stack.
    fn walk_sync_project(
        &mut self,
        project: &Arc<Project>,
        skip_affected: bool,
    ) -> miette::Result<bool> {
        // The projects on the current path (the stack), which are skipped
        // as dependencies, so that cycles across scopes are walked once
        let mut path = FxHashSet::default();

        let frame = match self.enter_sync_project(project, &path, skip_affected)? {
            SyncEntry::Skipped => return Ok(false),
            SyncEntry::Existing => return Ok(true),
            SyncEntry::New(frame) => frame,
        };

        path.insert(frame.project.id.clone());

        let mut stack = vec![frame];

        while let Some(frame) = stack.last_mut() {
            let Some(dep_id) = frame.deps.get(frame.next_dep).cloned() else {
                let frame = stack.pop().unwrap();

                path.remove(&frame.project.id);

                self.events
                    .push(Event::LinkSyncProject(frame.project.id.clone()));

                continue;
            };

            frame.next_dep += 1;

            if path.contains(&dep_id) {
                continue;
            }

            let dep_project = self.workspace_graph.get_project(&dep_id)?;

            if let SyncEntry::New(child) =
                self.enter_sync_project(&dep_project, &path, skip_affected)?
            {
                path.insert(child.project.id.clone());
                stack.push(child);
            }
        }

        Ok(true)
    }

    fn enter_sync_project(
        &mut self,
        project: &Arc<Project>,
        path: &FxHashSet<Id>,
        skip_affected: bool,
    ) -> miette::Result<SyncEntry> {
        // Explicitly disabled
        if !self.sync_projects.is_enabled(&project.id) || path.contains(&project.id) {
            return Ok(SyncEntry::Skipped);
        }

        // Return early if not affected
        if !self.is_project_affected(project, skip_affected)? {
            return Ok(SyncEntry::Skipped);
        }

        self.events.push(Event::SyncWorkspace);

        if !self.synced_project_ids.insert(project.id.clone()) {
            return Ok(SyncEntry::Existing);
        }

        self.events.push(Event::AddSyncProject(project.id.clone()));
        self.synced_projects.push(Arc::clone(project));

        Ok(SyncEntry::New(SyncFrame {
            project: Arc::clone(project),
            deps: if self.sync_project_dependencies {
                self.workspace_graph.projects.dependencies_of(project)
            } else {
                vec![]
            },
            next_dep: 0,
        }))
    }

    fn is_project_affected(
        &mut self,
        project: &Project,
        skip_affected: bool,
    ) -> miette::Result<bool> {
        if let Some(affected) = &mut self.affected
            && !skip_affected
        {
            // Short-circuit early if the project is already marked
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

    /// Whether the task is affected, marking it when it is. When affected
    /// checks are skipped, the task is still checked and marked, so that its
    /// affected state is reported, but it's always considered affected.
    fn is_task_affected(&mut self, task: &Task, skip_affected: bool) -> miette::Result<bool> {
        let Some(affected) = &mut self.affected else {
            // Always affected
            return Ok(true);
        };

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

        Ok(skip_affected)
    }

    // RESOLUTION

    /// Resolve the dependents of the provided tasks, up to the depth, in
    /// breadth-first order. The provided tasks are excluded. When a query has
    /// been set, dependents in projects that don't match it are not added,
    /// nor walked through. Internal dependents are walked through, but not
    /// added, as they only run as a dependency of another task.
    fn resolve_dependents(&self, tasks: &[&Task], depth: Depth) -> miette::Result<Vec<Arc<Task>>> {
        if depth.is_none() || tasks.is_empty() {
            return Ok(vec![]);
        }

        let query_project_ids = match self.query {
            Some(query) => Some(
                self.workspace_graph
                    .query_projects(query)?
                    .into_iter()
                    .map(|project| project.id.clone())
                    .collect::<FxHashSet<_>>(),
            ),
            None => None,
        };

        let mut targets = vec![];

        // Cleanup dependencies are dependents in the task graph, as they run
        // after the task, but they're always linked through the task itself,
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
                if !task.is_internal() {
                    targets.push(task.target.clone());
                }
            },
        );

        let mut dep_tasks = vec![];

        for target in targets {
            dep_tasks.extend(self.resolve_tasks_from_target(&target, TaskOrigin::Dependent)?);
        }

        Ok(dep_tasks)
    }

    /// Resolve the tasks of a dependency target, once.
    fn resolve_dependency_tasks(&mut self, target: &Target) -> miette::Result<DependencyTasks> {
        if let Some(tasks) = self.dependency_tasks.get(target) {
            return Ok(Arc::clone(tasks));
        }

        let tasks: DependencyTasks = self
            .resolve_tasks_from_target(target, TaskOrigin::Dependency)?
            .into();

        self.dependency_tasks
            .insert(target.to_owned(), Arc::clone(&tasks));

        Ok(tasks)
    }

    /// Resolve the tasks of a target. Requested tasks exclude internal tasks,
    /// and `:task` targets are bounded by the query. Dependents and dependencies
    /// are neither, as internal tasks can be depended on, and dependencies are
    /// always inserted.
    fn resolve_tasks_from_target(
        &self,
        target: &Target,
        origin: TaskOrigin,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];
        let requested = origin == TaskOrigin::Requested;

        // First, find all projects based on the target scope
        let (scope, scope_value) = target.get_project_scope();

        let (projects, bubble_error) = match scope {
            // :task
            TargetProjectScope::All => {
                let projects = match self.query {
                    Some(query) if requested => self.workspace_graph.query_projects(query)?,
                    _ => self.workspace_graph.get_projects()?,
                };

                (projects, false)
            }
            // project:task
            TargetProjectScope::Id => (vec![self.workspace_graph.get_project(scope_value)?], true),
            // #tag:task
            TargetProjectScope::Tag => (
                self.workspace_graph
                    .query_projects(build_query(format!("projectTag={scope_value}").as_str())?)?,
                false,
            ),
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
                            }

                            continue;
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
                if requested && project_task.is_internal() {
                    if bubble_error {
                        return Err(ProjectError::UnknownTask {
                            task_id: project_task.id.to_string(),
                            project_id: project.id.to_string(),
                        }
                        .into());
                    }

                    continue;
                }

                tasks.push(project_task);
            }
        }

        Ok(tasks)
    }

    fn resolve_tasks_from_target_locator(
        &self,
        locator: &TargetLocator,
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
                } else if let Some(TargetProjectScope::All) = project {
                    // Don't query for the other scopes,
                    // since they're not valid from the run context
                    is_all = true;
                    do_query = true;
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
                        if task.is_internal() {
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

                tasks.extend(self.resolve_tasks_from_target(&target, TaskOrigin::Requested)?);
            }
            TargetLocator::DefaultProject(task_id) => {
                let project = self.workspace_graph.get_default_project().map_err(|_| {
                    ProjectGraphError::NoDefaultProjectForTask {
                        task_id: task_id.to_string(),
                    }
                })?;

                let target = Target::new(&project.id, task_id)?;

                tasks.extend(self.resolve_tasks_from_target(&target, TaskOrigin::Requested)?);
            }
        };

        Ok(tasks)
    }
}
