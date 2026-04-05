use crate::projects_builder::ProjectBuildData;
use daggy::Dag;
use moon_common::Id;
use moon_config::TaskDependencyType;
use moon_graph_utils::NodeState;
use moon_task::{Target, Task, TaskOptions};
use moon_task_graph::TaskGraphError;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub type TaskDag = Dag<NodeState<Task>, TaskDependencyType>;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TaskBuildData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<NodeIndex>,

    #[serde(skip)]
    pub options: TaskOptions,
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
        Target::new_project(&project_id, &target.task_id)
    }
}

#[derive(Deserialize, Serialize)]
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
        Self {
            graph: TaskDag::default(),
            targets_to_indexes: FxHashMap::default(),
        }
    }

    #[instrument(skip_all)]
    pub fn build(&mut self, tasks: Vec<Task>) -> miette::Result<()> {
        for task in tasks {
            let from_index = self.get_or_insert_node(&task.target);

            for dep_config in &task.deps {
                let to_index = self.get_or_insert_node(&dep_config.target);
                let scope = if dep_config.optional.is_some_and(|v| v) {
                    TaskDependencyType::Optional
                } else {
                    TaskDependencyType::Required
                };

                self.graph
                    .add_edge(from_index, to_index, scope)
                    .map_err(|_| TaskGraphError::WouldCycle {
                        source_target: task.target.to_string(),
                        target_target: dep_config.target.to_string(),
                    })?;
            }

            self.insert_or_update_node(task);
        }

        Ok(())
    }
}
