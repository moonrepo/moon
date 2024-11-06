use moon_config::DependencyType;
use moon_target::Target;
use moon_task::Task;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_utils::json;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::debug;

pub type TaskGraphType = DiGraph<Target, DependencyType>;
pub type TasksCache = FxHashMap<Target, Arc<Task>>;

#[derive(Serialize)]
pub struct TaskGraphCache<'graph> {
    graph: &'graph TaskGraphType,
    tasks: &'graph TasksCache,
}

#[derive(Clone, Debug, Default)]
pub struct TaskNode {
    pub index: NodeIndex,
}

#[derive(Default)]
pub struct TaskGraph {
    /// Directed-acyclic graph (DAG) of non-expanded tasks and their relationships.
    graph: TaskGraphType,

    /// Graph node information, mapped by target.
    nodes: FxHashMap<Target, TaskNode>,

    /// Expanded tasks, mapped by target.
    tasks: Arc<RwLock<TasksCache>>,
}

impl TaskGraph {
    pub fn new(graph: TaskGraphType, nodes: FxHashMap<Target, TaskNode>) -> Self {
        debug!("Creating task graph");

        Self {
            graph,
            nodes,
            tasks: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    pub fn dependencies_of(&self, task: &Task) -> miette::Result<Vec<&Target>> {
        let deps = self
            .graph
            .neighbors_directed(
                self.nodes.get(&task.target).unwrap().index,
                Direction::Outgoing,
            )
            .map(|idx| self.graph.node_weight(idx).unwrap())
            .collect();

        Ok(deps)
    }

    pub fn dependents_of(&self, task: &Task) -> miette::Result<Vec<&Target>> {
        let deps = self
            .graph
            .neighbors_directed(
                self.nodes.get(&task.target).unwrap().index,
                Direction::Incoming,
            )
            .map(|idx| self.graph.node_weight(idx).unwrap())
            .collect();

        Ok(deps)
    }

    /// Return a list of targets for all tasks currently within the graph.
    pub fn targets(&self) -> Vec<&Target> {
        self.graph.raw_nodes().iter().map(|n| &n.weight).collect()
    }

    /// Get a labelled representation of the graph (which can be serialized easily).
    pub fn labeled_graph(&self) -> DiGraph<String, DependencyType> {
        self.graph.map(|_, n| n.to_string(), |_, e| *e)
    }

    /// Format graph as a DOT string.
    pub fn to_dot(&self) -> String {
        let dot = Dot::with_attr_getters(
            &self.graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                let label = e.weight().to_string();

                if e.source().index() == 0 {
                    format!("label=\"{label}\" arrowhead=none")
                } else {
                    format!("label=\"{label}\" arrowhead=box, arrowtail=box")
                }
            },
            &|_, n| {
                let label = &n.1.id;

                format!(
                    "label=\"{label}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black"
                )
            },
        );

        format!("{dot:?}")
    }

    /// Format graph as a JSON string.
    pub fn to_json(&self) -> miette::Result<String> {
        let tasks = self.read_cache();

        Ok(json::format(
            &TaskGraphCache {
                graph: &self.graph,
                tasks: &tasks,
            },
            true,
        )?)
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
