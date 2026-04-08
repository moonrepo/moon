use daggy::Dag;
use moon_config::TaskDependencyType;
use moon_graph_utils::*;
use moon_project::ProjectError;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_task::Task;
use moon_task_expander::TaskExpander;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use scc::hash_map::Entry;
use std::sync::Arc;
use tracing::{debug, instrument};

#[derive(Clone, Debug, Default)]
pub struct TaskNode {
    pub index: NodeIndex,
    pub task: Task,
}

#[derive(Debug, Default)]
pub struct TaskGraph {
    pub context: GraphExpanderContext,

    /// Directed-acyclic graph (DAG) of non-expanded tasks and their relationships.
    pub graph: Dag<NodeIndex, TaskDependencyType>,

    /// Map of node indexes to task targets.
    pub indexes: FxHashMap<NodeIndex, Target>,

    /// Map of task nodes by target.
    pub nodes: FxHashMap<Target, TaskNode>,

    /// Project graph, required for expansion.
    project_graph: Arc<ProjectGraph>,

    /// Map of expanded tasks by target.
    tasks: Arc<scc::HashMap<Target, Arc<Task>>>,
}

impl TaskGraph {
    pub fn new(context: GraphExpanderContext, project_graph: Arc<ProjectGraph>) -> Self {
        debug!("Creating task graph");

        Self {
            context,
            project_graph,
            ..Default::default()
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
        let node = self
            .nodes
            .get(target)
            .ok_or_else(|| ProjectError::UnknownTask {
                task_id: target.task_id.to_string(),
                project_id: target.get_project_id().unwrap().to_string(),
            })?;

        Ok(&node.task)
    }

    /// Return all tasks from the graph.
    #[instrument(name = "get_all_tasks", skip(self))]
    pub fn get_all(&self) -> miette::Result<Vec<Arc<Task>>> {
        let mut all = vec![];

        for target in self.nodes.keys() {
            all.push(self.internal_get(target)?);
        }

        Ok(all)
    }

    /// Return all unexpanded tasks from the graph.
    pub fn get_all_unexpanded(&self) -> miette::Result<Vec<&Task>> {
        Ok(self.nodes.values().map(|node| &node.task).collect())
    }

    /// Return many tasks from the graph by target.
    #[instrument(name = "get_many_tasks", skip(self))]
    pub fn get_many(&self, targets: &[Target]) -> miette::Result<Vec<Arc<Task>>> {
        let mut many = vec![];

        for target in targets {
            many.push(self.internal_get(target)?);
        }

        Ok(many)
    }

    /// Return many unexpanded tasks from the graph.
    pub fn get_many_unexpanded(&self, targets: &[Target]) -> miette::Result<Vec<&Task>> {
        let mut many = vec![];

        for target in targets {
            many.push(self.get_unexpanded(target)?);
        }

        Ok(many)
    }

    /// Focus the graph for a specific project by target.
    pub fn focus_for(&self, target: &Target, with_dependents: bool) -> miette::Result<Self> {
        let task = self.get(target)?;
        let graph = self.to_focused_graph(&task, with_dependents);
        let (nodes, edges) = graph.into_nodes_edges();

        let mut dag = Dag::with_capacity(nodes.len(), edges.len());
        let mut indexes = FxHashMap::default();
        let mut tasks = FxHashMap::default();

        // The focused graph has different node inndexes,
        // so we need to update our internal structures to match
        for (i, node) in nodes.into_iter().enumerate() {
            let new_index = NodeIndex::from(i as u32);
            let old_index = node.weight;
            let target = &self.indexes[&old_index];

            indexes.insert(new_index, target.to_owned());

            tasks.insert(
                target.to_owned(),
                TaskNode {
                    index: new_index,
                    task: self.get_node_by_index(&old_index).to_owned(),
                },
            );

            dag.add_node(new_index);
        }

        for edge in edges {
            dag.update_edge(edge.source(), edge.target(), edge.weight)
                .unwrap();
        }

        Ok(Self {
            indexes,
            context: self.context.clone(),
            graph: dag,
            nodes: tasks,
            project_graph: self.project_graph.clone(),
            tasks: self.tasks.clone(),
        })
    }

    fn internal_get(&self, target: &Target) -> miette::Result<Arc<Task>> {
        let task = match self.tasks.entry_sync(target.to_owned()) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                let expander = TaskExpander::new(
                    &self.project_graph,
                    self.project_graph
                        .get_unexpanded(target.get_project_id()?)?,
                    &self.context,
                );

                let task = Arc::new(expander.expand(self.get_unexpanded(entry.key())?)?);

                entry.insert_entry(Arc::clone(&task));

                task
            }
        };

        Ok(task)
    }
}

impl GraphData<Task, TaskDependencyType, Target> for TaskGraph {
    fn get_graph(&self) -> &DiGraph<NodeIndex, TaskDependencyType> {
        self.graph.graph()
    }

    fn get_nodes(&self) -> FxHashMap<NodeIndex, &Task> {
        self.nodes
            .values()
            .map(|node| (node.index, &node.task))
            .collect()
    }

    fn get_node_by_index(&self, index: &NodeIndex) -> &Task {
        &self.nodes[&self.indexes[index]].task
    }

    fn get_node_key(&self, node: &Task) -> Target {
        node.target.clone()
    }
}

impl GraphConnections<Task, TaskDependencyType, Target> for TaskGraph {
    fn get_node_index(&self, node: &Task) -> NodeIndex {
        self.nodes[&node.target].index
    }
}

impl GraphConversions<Task, TaskDependencyType, Target> for TaskGraph {}

impl GraphToDot<Task, TaskDependencyType, Target> for TaskGraph {}

impl GraphToJson<Task, TaskDependencyType, Target> for TaskGraph {}
