use crate::task_graph_error::TaskGraphError;
use moon_config::DependencyType;
use moon_graph_utils::*;
use moon_target::Target;
use moon_task::Task;
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
    /// Directed-acyclic graph (DAG) of non-expanded tasks and their relationships.
    graph: TaskGraphType,

    /// Task metadata, mapped by target.
    metadata: FxHashMap<Target, TaskMetadata>,

    /// Expanded tasks, mapped by target.
    tasks: Arc<RwLock<TasksCache>>,
}

impl TaskGraph {
    pub fn new(graph: TaskGraphType, metadata: FxHashMap<Target, TaskMetadata>) -> Self {
        debug!("Creating task graph");

        Self {
            graph,
            metadata,
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

    /// Return all unexpanded tasks from the graph.
    pub fn get_all_unexpanded(&self) -> Vec<&Task> {
        self.graph
            .raw_nodes()
            .iter()
            .map(|node| &node.weight)
            .collect()
    }

    // fn internal_get(&self, target: &Target) -> miette::Result<Arc<Task>> {
    //     // Check if the expanded task has been created, if so return it
    //     if let Some(task) = self.read_cache().get(target) {
    //         return Ok(Arc::clone(task));
    //     }

    //     // Otherwise expand the project and cache it with an Arc
    //     let query = |input: String| {
    //         let mut results = vec![];

    //         // Don't use get() for expanded projects, since it'll overflow the
    //         // stack trying to recursively expand projects! Using unexpanded
    //         // dependent projects works just fine for the this entire process.
    //         for result_id in self.internal_query(build_query(&input)?)?.iter() {
    //             results.push(self.get_unexpanded(result_id)?);
    //         }

    //         Ok(results)
    //     };

    //     let expander = ProjectExpander::new(ExpanderContext {
    //         aliases: self.aliases(),
    //         project: self.get_unexpanded(&id)?,
    //         query: Box::new(query),
    //         workspace_root: &self.workspace_root,
    //     });

    //     let project = Arc::new(expander.expand()?);

    //     self.write_cache().insert(id.clone(), Arc::clone(&project));

    //     Ok(project)
    // }

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
