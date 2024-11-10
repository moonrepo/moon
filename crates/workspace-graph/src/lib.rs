use moon_project_graph::ProjectGraph;
use moon_task_graph::TaskGraph;
use std::sync::Arc;

pub struct WorkspaceGraph {
    pub projects: Arc<ProjectGraph>,
    pub tasks: Arc<TaskGraph>,
}
