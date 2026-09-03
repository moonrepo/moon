use crate::projects_builder::ProjectBuildData;
use daggy::Dag;
use moon_common::Id;
use moon_config::{TaskDependencyConfig, TaskDependencyType};
use moon_graph_utils::{GraphExpanderContext, NodeState};
use moon_project_graph::ProjectGraph;
use moon_task::{Target, Task, TaskOptions};
use moon_task_graph::{TaskGraph, TaskGraphError, TaskNode};
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

pub type TaskDag = Dag<NodeState<Task>, TaskDependencyType>;

/// Resolve the edge weight for a task dependency. The configured type always
/// wins, except for `required` dependencies that were inherited as optional,
/// which are marked with the internal-only `optional` weight.
pub fn resolve_dep_edge_type(dep_config: &TaskDependencyConfig) -> TaskDependencyType {
    if dep_config.type_of.is_required_type() && dep_config.optional.is_some_and(|v| v) {
        TaskDependencyType::Optional
    } else {
        dep_config.type_of
    }
}

/// Return the graph edge endpoints for a task dependency, based on its type.
/// `cleanup` dependencies run *after* the task, so their edge is reversed —
/// the cleanup task depends on the task that it cleans up.
pub fn resolve_dep_edge_endpoints(
    task_index: NodeIndex,
    dep_index: NodeIndex,
    edge_type: TaskDependencyType,
) -> (NodeIndex, NodeIndex) {
    if matches!(edge_type, TaskDependencyType::Cleanup) {
        (dep_index, task_index)
    } else {
        (task_index, dep_index)
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TaskBuildData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<NodeIndex>,

    #[serde(skip)]
    pub options: TaskOptions,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Id>,

    #[serde(skip)]
    pub has_outputs: bool,
}

impl TaskBuildData {
    // TODO deprecated
    pub fn resolve_target(
        target: &Target,
        project_data: &FxHashMap<Id, ProjectBuildData>,
    ) -> miette::Result<Target> {
        // Target may be using an alias!
        let project_id = ProjectBuildData::resolve_id(target.get_project_id()?, project_data);

        // IDs should be valid here, so ignore the result
        Target::new(&project_id, target.get_task_id()?)
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct WorkspaceTasksBuilder {
    /// The task DAG.
    pub graph: TaskDag,

    /// Map of task targets to their graph index.
    pub targets_to_indexes: FxHashMap<Target, NodeIndex>,
}

impl WorkspaceTasksBuilder {
    pub fn get_or_insert_node(&mut self, target: &Target) -> NodeIndex {
        match self.targets_to_indexes.get(target) {
            Some(index) => *index,
            None => {
                let index = self.graph.add_node(NodeState::Loading);
                self.targets_to_indexes.insert(target.to_owned(), index);
                index
            }
        }
    }

    pub fn insert_or_update_node(&mut self, task: Task) {
        // Project node may have been inserted through an edge first,
        // so we need to update the state from loading to loaded
        if let Some(index) = self.targets_to_indexes.get(&task.target)
            && let Some(node) = self.graph.node_weight_mut(*index)
        {
            *node = NodeState::Loaded(task);
        }
        // Otherwise the node was inserted first, so we can set as loaded
        else {
            self.targets_to_indexes.insert(
                task.target.clone(),
                self.graph.add_node(NodeState::Loaded(task)),
            );
        }
    }
}

impl WorkspaceTasksBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[instrument(skip_all)]
    pub fn build(&mut self, tasks: Vec<Task>) -> miette::Result<()> {
        for task in tasks {
            let from_index = self.get_or_insert_node(&task.target);

            for dep_config in &task.deps {
                let dep_index = self.get_or_insert_node(&dep_config.target);
                let edge_type = resolve_dep_edge_type(dep_config);
                let (source_index, target_index) =
                    resolve_dep_edge_endpoints(from_index, dep_index, edge_type);

                self.graph
                    .add_edge(source_index, target_index, edge_type)
                    .map_err(|_| {
                        let (source, target) = if source_index == from_index {
                            (&task.target, &dep_config.target)
                        } else {
                            (&dep_config.target, &task.target)
                        };

                        TaskGraphError::WouldCycle {
                            source_target: source.to_string(),
                            target_target: target.to_string(),
                        }
                    })?;
            }

            self.insert_or_update_node(task);
        }

        Ok(())
    }

    pub fn finalize(
        self,
        context: GraphExpanderContext,
        project_graph: Arc<ProjectGraph>,
    ) -> TaskGraph {
        let mut task_graph = TaskGraph::new(context, project_graph);
        let mut loaded_tasks = FxHashMap::default();

        // TODO switch to filter_map_owned
        task_graph.graph = self.graph.filter_map(
            |ni, node| match node {
                NodeState::Loading => None,
                NodeState::Loaded(task) => {
                    loaded_tasks.insert(ni, task.to_owned());

                    Some(ni)
                }
            },
            |_, edge| Some(*edge),
        );

        for index in task_graph.graph.graph().node_indices() {
            let old_index = *task_graph.graph.node_weight(index).unwrap();
            let task = loaded_tasks.remove(&old_index).unwrap();
            let target = task.target.clone();

            task_graph.indexes.insert(index, target.clone());
            task_graph.nodes.insert(target, TaskNode { index, task });
        }

        // Weight-based lookups require each node's weight to be its own
        // index, which may not be the case when placeholder nodes were
        // dropped by the filter above, so rewrite them
        for index in 0..task_graph.graph.node_count() {
            let index = NodeIndex::new(index);
            *task_graph.graph.node_weight_mut(index).unwrap() = index;
        }

        task_graph
    }
}
