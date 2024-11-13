use crate::task_graph_error::TaskGraphError;
use moon_common::Id;
use moon_config::DependencyType;
use moon_graph_utils::*;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_task::Task;
use moon_task_expander::TaskExpander;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, instrument};

pub type TaskGraphType = DiGraph<Task, DependencyType>;
pub type TasksCache = FxHashMap<Target, Arc<Task>>;

#[derive(Clone, Debug, Default)]
pub struct TaskMetadata {
    pub index: NodeIndex,
}

#[derive(Default)]
pub struct TaskGraph {
    context: GraphExpanderContext,

    /// Directed-acyclic graph (DAG) of non-expanded tasks and their relationships.
    graph: TaskGraphType,

    /// Task metadata, mapped by target.
    metadata: FxHashMap<Target, TaskMetadata>,

    /// Project graph, required for expansion.
    project_graph: Arc<ProjectGraph>,

    /// Expanded tasks, mapped by target.
    tasks: Arc<RwLock<TasksCache>>,
}

impl TaskGraph {
    pub fn new(
        graph: TaskGraphType,
        metadata: FxHashMap<Target, TaskMetadata>,
        context: GraphExpanderContext,
        project_graph: Arc<ProjectGraph>,
    ) -> Self {
        debug!("Creating task graph");

        Self {
            context,
            graph,
            metadata,
            project_graph,
            tasks: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Return a task with the provided target from the graph.
    /// If the task does not exist or has been misconfigured, return an error.
    #[instrument(name = "get_task", skip(self))]
    pub fn get(&self, target: &Target) -> miette::Result<Arc<Task>> {
        self.internal_get(target)
    }

    /// Return an unexpanded task with the provided target from the graph.
    pub fn get_unexpanded(&self, target: &Target) -> miette::Result<&Task> {
        let metadata = self
            .metadata
            .get(target)
            .ok_or(TaskGraphError::UnconfiguredTarget(target.to_owned()))?;

        Ok(self.graph.node_weight(metadata.index).unwrap())
    }

    /// Return all tasks from the graph.
    #[instrument(name = "get_all_tasks", skip(self))]
    pub fn get_all(&self) -> miette::Result<Vec<Arc<Task>>> {
        let mut all = vec![];

        for target in self.metadata.keys() {
            all.push(self.internal_get(target)?);
        }

        Ok(all)
    }

    /// Return all tasks for a specific project from the graph.
    #[instrument(name = "get_all_project_tasks", skip(self))]
    pub fn get_all_for_project(
        &self,
        project_id: &Id,
        include_internal: bool,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut all = vec![];

        for target in self.metadata.keys() {
            if target.get_project_id().is_some_and(|id| id == project_id) {
                let task = self.internal_get(target)?;

                if !include_internal && task.is_internal() {
                    continue;
                }

                all.push(task);
            }
        }

        Ok(all)
    }

    /// Return all unexpanded tasks from the graph.
    pub fn get_all_unexpanded(&self) -> Vec<&Task> {
        self.graph
            .raw_nodes()
            .iter()
            .map(|node| &node.weight)
            .collect()
    }

    /// Focus the graph for a specific project by target.
    pub fn focus_for(&self, target: &Target, with_dependents: bool) -> miette::Result<Self> {
        let task = self.get(target)?;
        let graph = self.to_focused_graph(&task, with_dependents);

        // Copy over metadata
        let mut metadata = FxHashMap::default();

        for new_index in graph.node_indices() {
            let inner_target = &graph[new_index].target;

            if let Some(old_node) = self.metadata.get(inner_target) {
                let mut new_node = old_node.to_owned();
                new_node.index = new_index;

                metadata.insert(inner_target.to_owned(), new_node);
            }
        }

        Ok(Self {
            context: self.context.clone(),
            graph,
            metadata,
            project_graph: self.project_graph.clone(),
            tasks: self.tasks.clone(),
        })
    }

    fn internal_get(&self, target: &Target) -> miette::Result<Arc<Task>> {
        if let Some(task) = self.read_cache().get(target) {
            return Ok(Arc::clone(task));
        }

        let mut cache = self.write_cache();

        let expander = TaskExpander::new(
            self.project_graph.get_unexpanded(
                target
                    .get_project_id()
                    .expect("Project scope required for target."),
            )?,
            &self.context,
        );

        let task = Arc::new(expander.expand(self.get_unexpanded(target)?)?);

        cache.insert(target.to_owned(), Arc::clone(&task));

        Ok(task)
    }

    fn read_cache(&self) -> RwLockReadGuard<TasksCache> {
        self.tasks
            .read()
            .expect("Failed to acquire read access to task graph!")
    }

    fn write_cache(&self) -> RwLockWriteGuard<TasksCache> {
        self.tasks
            .write()
            .expect("Failed to acquire write access to task graph!")
    }
}

impl GraphData<Task, DependencyType, Target> for TaskGraph {
    fn get_graph(&self) -> &DiGraph<Task, DependencyType> {
        &self.graph
    }

    fn get_node_index(&self, node: &Task) -> NodeIndex {
        self.metadata.get(&node.target).unwrap().index
    }

    fn get_node_key(&self, node: &Task) -> Target {
        node.target.clone()
    }
}

impl GraphConnections<Task, DependencyType, Target> for TaskGraph {}

impl GraphConversions<Task, DependencyType, Target> for TaskGraph {}

impl GraphToDot<Task, DependencyType, Target> for TaskGraph {}

impl GraphToJson<Task, DependencyType, Target> for TaskGraph {}
