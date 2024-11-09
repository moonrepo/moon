use moon_config::DependencyType;
use moon_graph_utils::*;
use moon_target::Target;
use moon_task::Task;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use std::sync::Arc;
// use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::debug;

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
    // /// Expanded tasks, mapped by target.
    // tasks: Arc<RwLock<TasksCache>>,
}

impl TaskGraph {
    pub fn new(graph: TaskGraphType, metadata: FxHashMap<Target, TaskMetadata>) -> Self {
        debug!("Creating task graph");

        Self {
            graph,
            metadata,
            // tasks: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    // fn read_cache(&self) -> RwLockReadGuard<TasksCache> {
    //     self.tasks
    //         .read()
    //         .expect("Failed to acquire read access to task graph!")
    // }

    // fn write_cache(&self) -> RwLockWriteGuard<TasksCache> {
    //     self.tasks
    //         .write()
    //         .expect("Failed to acquire write access to task graph!")
    // }
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
