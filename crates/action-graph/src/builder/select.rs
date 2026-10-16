//! Stage 1: select the task instances that exist, and how they relate.
//!
//! This doesn't create any graph nodes or edges. It walks the requests, and the
//! dependencies of each task they reach, applying the same gates in the same
//! order as the action graph has always done, and records the result as data:
//! the instances (in discovery order), their relations (in the order they're
//! linked), and the serial ordering links (in the order they're made).

use super::RequestId;
use crate::action_graph_builder::{RunRequirements, TaskOrigin};
use crate::action_graph_error::ActionGraphError;
use moon_action::{ActionNode, RunTaskNode};
use moon_affected::{AffectedTracker, Depth};
use moon_common::color;
use moon_config::{EnvMap, TaskDependencyConfig, TaskDependencyType};
use moon_exec_plan::TargetsBlock;
use moon_project::ProjectError;
use moon_query::{Criteria, build_query};
use moon_task::{Target, TargetError, TargetLocator, TargetProjectScope, TargetTaskScope, Task};
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

/// A relation between 2 task instances, from the instance that runs later.
#[derive(Debug)]
pub struct TaskRelation {
    pub from: usize,
    pub to: usize,
    pub weight: TaskDependencyType,
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
}

/// Everything selected for the task requests of a build.
pub struct TaskPlan {
    /// The `RunTask` payload of each instance, in discovery order.
    pub instances: Vec<ActionNode>,
    /// Relations in the order that they're linked: a task's requirements once
    /// all of its dependencies have been visited, then its cleanups.
    pub relations: Vec<TaskRelation>,
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

/// A task whose dependencies are being visited.
struct Frame {
    instance: usize,
    parallel: bool,
    deps: Vec<TaskDependencyConfig>,
    next_dep: usize,
    /// The dependency being visited, its resolved tasks, and the next one.
    current: Option<(usize, Vec<Arc<Task>>, usize)>,
    /// A dependency instance that was just created, and is linked once its own
    /// dependencies have been visited (when its frame is popped).
    pending: Option<(usize, Arc<Task>, usize)>,
    previous: Option<usize>,
    previous_standard: Option<usize>,
    requirements: Vec<(usize, TaskDependencyType)>,
    cleanups: Vec<usize>,
}

enum Entry {
    /// The task didn't pass a gate.
    Skipped,
    /// An instance with the same identity already exists.
    Existing(usize),
    /// A new instance, and a frame when it has dependencies to visit.
    New(usize, Option<Frame>),
}

pub struct TaskSelector<'query> {
    workspace_graph: Arc<WorkspaceGraph>,
    query: Option<&'query Criteria<'query>>,
    affected: Option<AffectedTracker>,
    affected_depth: Depth,

    instances: Vec<ActionNode>,
    instance_indices: FxHashMap<TaskInstanceKey, usize>,
    relations: Vec<TaskRelation>,
    serial_links: Vec<SerialLink>,
    no_dependencies_tasks: FxHashMap<Target, Vec<TaskDependencyConfig>>,
    cleanup_targets: FxHashSet<Target>,
    passthrough_targets: FxHashSet<Target>,
    primary_targets: FxHashSet<Target>,
    wait_targets: FxHashSet<Target>,
    results: FxHashMap<RequestId, RequestResult>,
}

impl<'query> TaskSelector<'query> {
    pub fn new(
        workspace_graph: Arc<WorkspaceGraph>,
        query: Option<&'query Criteria<'query>>,
        affected: Option<AffectedTracker>,
        affected_depth: Depth,
    ) -> Self {
        Self {
            workspace_graph,
            query,
            affected,
            affected_depth,
            instances: vec![],
            instance_indices: FxHashMap::default(),
            relations: vec![],
            serial_links: vec![],
            no_dependencies_tasks: FxHashMap::default(),
            cleanup_targets: FxHashSet::default(),
            passthrough_targets: FxHashSet::default(),
            primary_targets: FxHashSet::default(),
            wait_targets: FxHashSet::default(),
            results: FxHashMap::default(),
        }
    }

    pub fn finish(self) -> TaskPlan {
        TaskPlan {
            instances: self.instances,
            relations: self.relations,
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

    /// Run the tasks of the provided target (and their dependents).
    pub fn run_task_by_target(
        &mut self,
        id: RequestId,
        target: &Target,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let tasks = self.resolve_tasks_from_target(target, false)?;

        self.run_primary_tasks(id, tasks, reqs)
    }

    /// Run the tasks of the provided locator (and their dependents).
    pub fn run_task_by_target_locator(
        &mut self,
        id: RequestId,
        locator: &TargetLocator,
        reqs: &RunRequirements,
    ) -> miette::Result<()> {
        let tasks = self.resolve_tasks_from_target_locator(locator, false)?;

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
            tasks.extend(self.resolve_tasks_from_target_locator(locator, false)?);
        }

        // Expand the selection with the dependents of the selected tasks up
        // front, so that the candidates don't depend on which tasks have been
        // inserted (or skipped) by the time the dependents are expanded
        let mut candidates = self.collect_candidates(tasks, reqs.dependents)?;

        // Determine the affected state of each task up front, as marking
        // lazily while inserting would depend on the order of the targets
        let affected_depth = self.affected_depth;

        if !reqs.skip_affected
            && let Some(affected) = &mut self.affected
        {
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
            let (start, stop) = if job_index == 0 {
                (0, job_size)
            } else if job_index == job_total - 1 {
                (job_size * job_index, candidates.len())
            } else {
                (job_size * job_index, job_size * (job_index + 1))
            };

            if candidates.get(start).is_some() {
                if candidates.get(stop).is_some() {
                    candidates = candidates[start..stop].to_vec();
                } else {
                    candidates = candidates[start..].to_vec();
                }
            }

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
        let index = self.insert(task, reqs, origin)?;

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
        reqs: &RunRequirements,
        origin: TaskOrigin,
    ) -> miette::Result<Option<usize>> {
        let (root, frame) = match self.enter(task, None, origin, reqs.skip_affected, reqs)? {
            Entry::Skipped => return Ok(None),
            Entry::Existing(index) => return Ok(Some(index)),
            Entry::New(index, frame) => (index, frame),
        };

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

                let Some(dep) = frame.deps.get(frame.next_dep) else {
                    break None;
                };

                let dep_tasks = self.resolve_tasks_from_target(&dep.target, true)?;

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
            let config = frame.deps[dep_index].clone();

            match self.enter(&dep_task, Some(&config), TaskOrigin::Dependency, true, reqs)? {
                Entry::Skipped => {}
                Entry::Existing(instance) | Entry::New(instance, None) => {
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
        self.workspace_graph
            .get_project(task.target.get_project_id()?)?;

        // Abort early if not affected
        if !self.is_task_affected(task, skip_affected)? {
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

        if task.deps.is_empty() {
            return Ok(Entry::New(index, None));
        }

        let deps = if reqs.no_dependencies {
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

            cleanups
        } else {
            task.deps.clone()
        };

        Ok(Entry::New(
            index,
            Some(Frame {
                instance: index,
                parallel: task.options.run_deps_in_parallel,
                deps,
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
        let type_of = frame.deps[dep_index].type_of;

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
        for (to, weight) in frame.requirements {
            self.relations.push(TaskRelation {
                from: frame.instance,
                to,
                weight,
            });
        }

        for from in frame.cleanups {
            self.relations.push(TaskRelation {
                from,
                to: frame.instance,
                weight: TaskDependencyType::Cleanup,
            });
        }
    }

    fn is_task_affected(&mut self, task: &Task, skip_affected: bool) -> miette::Result<bool> {
        if let Some(affected) = &mut self.affected
            && !skip_affected
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

    // RESOLUTION

    /// Resolve the dependents of the provided tasks, up to the depth, in
    /// breadth-first order. The provided tasks are excluded. When a query has
    /// been set, dependents in projects that don't match it are not added,
    /// nor walked through.
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
                targets.push(task.target.clone());
            },
        );

        let mut dep_tasks = vec![];

        for target in targets {
            dep_tasks.extend(self.resolve_tasks_from_target(&target, true)?);
        }

        Ok(dep_tasks)
    }

    fn resolve_tasks_from_target(
        &self,
        target: &Target,
        allow_internal: bool,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];

        // First, find all projects based on the target scope
        let (scope, scope_value) = target.get_project_scope();

        let (projects, bubble_error) = match scope {
            // :task
            TargetProjectScope::All => {
                let projects = match self.query {
                    Some(query) => self.workspace_graph.query_projects(query)?,
                    None => self.workspace_graph.get_projects()?,
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
                if !allow_internal && project_task.is_internal() {
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

                tasks.extend(self.resolve_tasks_from_target(&target, allow_internal)?);
            }
            TargetLocator::DefaultProject(task_id) => {
                let project = self.workspace_graph.get_default_project().map_err(|_| {
                    ProjectGraphError::NoDefaultProjectForTask {
                        task_id: task_id.to_string(),
                    }
                })?;

                let target = Target::new(&project.id, task_id)?;

                tasks.extend(self.resolve_tasks_from_target(&target, allow_internal)?);
            }
        };

        Ok(tasks)
    }
}
