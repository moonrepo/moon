use std::sync::Arc;

use crate::WorkspaceBuilderContext;
use crate::project_builder::ProjectBuildData;
use daggy::Dag;
use moon_common::Id;
use moon_config::TaskDependencyType;
use moon_task::{Target, Task, TaskOptions};
use moon_task_graph::NodeState;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TaskBuildData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<NodeIndex>,

    #[serde(skip)]
    pub options: TaskOptions,
}

impl TaskBuildData {
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

pub enum TaskBuildEvent {
    Node(Task),
    Edge(Target, Target, TaskDependencyType),
}

pub async fn build_task(
    _context: Arc<WorkspaceBuilderContext>,
    task: Task,
    tx: mpsc::Sender<TaskBuildEvent>,
) -> miette::Result<()> {
    // Send an event for each task-to-task relationship
    for dep_config in &task.deps {
        tx.send(TaskBuildEvent::Edge(
            task.target.clone(),
            dep_config.target.clone(),
            if dep_config.optional.is_some_and(|v| v) {
                TaskDependencyType::Optional
            } else {
                TaskDependencyType::Required
            },
        ))
        .await
        .expect("TODO");
    }

    // Send a final event for the task itself
    tx.send(TaskBuildEvent::Node(task)).await.expect("TODO");

    Ok(())
}

pub fn get_or_insert_task_node(
    target: &Target,
    graph: &mut Dag<NodeState<Task>, TaskDependencyType>,
    indexes: &mut FxHashMap<Target, NodeIndex>,
) -> NodeIndex {
    if let Some(index) = indexes.get(target) {
        *index
    } else {
        let index = graph.add_node(NodeState::Loading);
        indexes.insert(target.to_owned(), index);
        index
    }
}

pub fn insert_or_update_task_node(
    task: Task,
    graph: &mut Dag<NodeState<Task>, TaskDependencyType>,
    indexes: &mut FxHashMap<Target, NodeIndex>,
) {
    // Task node may have been inserted through an edge first,
    // so we need to update the state from loading to loaded
    if let Some(index) = indexes.get(&task.target)
        && let Some(node) = graph.node_weight_mut(*index)
    {
        *node = NodeState::Loaded(task);
    }
    // Otherwise the node was inserted first, so we can set as loaded
    else {
        indexes.insert(task.target.clone(), graph.add_node(NodeState::Loaded(task)));
    }
}
