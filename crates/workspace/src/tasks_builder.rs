use crate::projects_builder::{ProjectBuildData, WorkspaceProjectsBuilder};
use crate::workspace_builder::WorkspaceBuilderContext;
use daggy::Dag;
use moon_common::{Id, color};
use moon_config::TaskDependencyType;
use moon_graph_utils::NodeState;
use moon_task::{Target, Task, TaskOptions};
use moon_task_graph::TaskGraphError;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::mem;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

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

#[derive(Debug)]
pub enum TaskBuildEvent {
    Edge(Target, Target, TaskDependencyType),
    Node(Arc<Task>),
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
    tx.send(TaskBuildEvent::Node(Arc::new(task)))
        .await
        .expect("TODO");

    Ok(())
}

#[derive(Deserialize, Serialize)]
pub struct WorkspaceTasksBuilder {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext>>,

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
    pub fn new(context: Arc<WorkspaceBuilderContext>) -> Self {
        Self {
            context: Some(context),
            graph: TaskDag::default(),
            targets_to_indexes: FxHashMap::default(),
        }
    }

    /// Load and build all projects into the graph, as configured in the workspace.
    #[instrument(skip(self, projects))]
    pub async fn build(&mut self, projects: &mut WorkspaceProjectsBuilder) -> miette::Result<()> {
        let tasks = self.extract_tasks(projects)?;

        self.build_graph(tasks).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn build_graph(&mut self, tasks: Vec<Task>) -> miette::Result<()> {
        let context = self.context();
        let mut set = JoinSet::new();
        let (tx, mut rx) = mpsc::channel::<TaskBuildEvent>(1000);

        // Build each task in a separate task
        for task in tasks {
            debug!(
                task_target = task.target.as_str(),
                "Building task {}",
                color::id(&task.target)
            );

            set.spawn(Box::pin(build_task(Arc::clone(&context), task, tx.clone())));
        }

        // Receive events from each background task
        drop(tx);

        while let Some(event) = rx.recv().await {
            match event {
                TaskBuildEvent::Node(task) => {
                    self.insert_or_update_node(Arc::unwrap_or_clone(task));
                }
                TaskBuildEvent::Edge(from_target, to_target, scope) => {
                    let from_index = self.get_or_insert_node(&from_target);
                    let to_index = self.get_or_insert_node(&to_target);

                    self.graph
                        .add_edge(from_index, to_index, scope)
                        .map_err(|_| TaskGraphError::WouldCycle {
                            source_target: from_target.to_string(),
                            target_target: to_target.to_string(),
                        })?;
                }
            }
        }

        // Ensure all background tasks have completed
        set.join_all().await;

        Ok(())
    }

    // Extract all tasks from their respective project, as the data will live
    // in the task graph and not the project graph!
    pub fn extract_tasks(
        &self,
        projects: &mut WorkspaceProjectsBuilder,
    ) -> miette::Result<Vec<Task>> {
        let mut tasks = vec![];

        for state in projects.graph.node_weights_mut() {
            if let NodeState::Loaded(project) = state {
                tasks.extend(mem::take(&mut project.tasks).into_values());
            }
        }

        Ok(tasks)
    }

    fn context(&self) -> Arc<WorkspaceBuilderContext> {
        Arc::clone(
            self.context
                .as_ref()
                .expect("Missing workspace builder context!"),
        )
    }
}
