use crate::projects_builder::*;
use crate::tasks_builder::*;
use crate::workspace_builder::*;
use daggy::Dag;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::TaskDependencyType;
use moon_task::{Target, Task};
use moon_task_graph::NodeState;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilderAsync {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext>>,

    /// List of config paths used in the hashing process.
    /// These are used for invalidation.
    config_paths: FxHashSet<WorkspaceRelativePathBuf>,

    /// Builder for everything projects related.
    projects: WorkspaceProjectsBuilder,

    /// Mapping of task targets to associated data required for building
    /// the project itself. Data is wiped after building the graph!
    task_data: FxHashMap<Target, TaskBuildData>,

    /// The task DAG.
    task_graph: Dag<NodeState<Task>, TaskDependencyType>,

    // Mapping of task targets to their node index in the graph, for quick lookup.
    task_indexes: FxHashMap<Target, NodeIndex>,
}

impl WorkspaceBuilderAsync {
    pub async fn new(context: WorkspaceBuilderContext) -> miette::Result<WorkspaceBuilderAsync> {
        debug!("Building workspace graph (project and task graphs)");

        let context = Arc::new(context);

        Ok(WorkspaceBuilderAsync {
            config_paths: FxHashSet::default(),
            projects: WorkspaceProjectsBuilder::new(Arc::clone(&context)),
            task_data: FxHashMap::default(),
            task_graph: Dag::new(),
            task_indexes: FxHashMap::default(),
            context: Some(context),
        })
    }

    /// Load and build all projects into the graph, as configured in the workspace.
    pub async fn build_project_graph(&mut self) -> miette::Result<()> {
        self.projects.build().await?;

        Ok(())
    }

    /// Load and build all tasks into the graph, as configured in the workspace.
    pub async fn build_task_graph(&mut self) -> miette::Result<()> {
        // let context = self.context();
        // let mut set = JoinSet::new();
        // let (tx, mut rx) = mpsc::channel::<TaskBuildEvent>(1000);

        // // Build each task in a separate task
        // for (target, _build_data) in mem::take(&mut self.task_data) {
        //     debug!(
        //         task_target = target.as_str(),
        //         "Building task {}",
        //         color::id(&target)
        //     );

        //     // Extract the task from the project, as the data will live
        //     // in the task graph and not the project graph
        //     let Some(project_index) = self.project_indexes.get(target.get_project_id()?) else {
        //         panic!("Unable to load task, owning project does not exist!");
        //     };

        //     let Some(NodeState::Loaded(project)) =
        //         self.project_graph.node_weight_mut(*project_index)
        //     else {
        //         panic!("Unable to load task, owning project is in a non-loaded state!");
        //     };

        //     let mut task = project.tasks.remove(&target.task_id).unwrap();

        //     // Resolve the task dependencies so we can link edges correctly
        //     TaskDepsBuilder {
        //         querent: Box::new(WorkspaceBuilderTasksQuerent {
        //             project_data: &self.project_data,
        //             projects_by_tag: &self.projects_by_tag,
        //             task_data: &self.task_data,
        //         }),
        //         project: Some(project),
        //         root_project_id: self.root_project_id.as_ref(),
        //         task: &mut task,
        //     }
        //     .build()?;

        //     let context = Arc::clone(&context);
        //     let tx = tx.clone();

        //     set.spawn(async move { build_task(context, task, tx).await });
        // }

        // // Receive events from each background task
        // while let Some(event) = rx.recv().await {
        //     match event {
        //         TaskBuildEvent::Node(task) => {
        //             insert_or_update_task_node(task, &mut self.task_graph, &mut self.task_indexes);
        //         }
        //         TaskBuildEvent::Edge(from_target, to_target, scope) => {
        //             let from_index = get_or_insert_task_node(
        //                 &from_target,
        //                 &mut self.task_graph,
        //                 &mut self.task_indexes,
        //             );

        //             let to_index = get_or_insert_task_node(
        //                 &to_target,
        //                 &mut self.task_graph,
        //                 &mut self.task_indexes,
        //             );

        //             self.task_graph
        //                 .add_edge(from_index, to_index, scope)
        //                 .map_err(|_| ProjectGraphError::WouldCycle {
        //                     source_id: from_target.to_string(),
        //                     target_id: to_target.to_string(),
        //                 })?;
        //         }
        //     }
        // }

        // // Ensure all background tasks have completed
        // set.join_all().await;

        Ok(())
    }

    fn context(&self) -> Arc<WorkspaceBuilderContext> {
        Arc::clone(
            self.context
                .as_ref()
                .expect("Missing workspace builder context!"),
        )
    }
}
