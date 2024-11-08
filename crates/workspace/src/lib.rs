mod build_data;
mod projects_locator;
mod repo_type;
mod tasks_querent;
mod workspace_builder;
mod workspace_builder_error;
mod workspace_cache;

pub use build_data::*;
pub use repo_type::*;
pub use tasks_querent::*;
pub use workspace_builder::*;
pub use workspace_builder_error::*;
pub use workspace_cache::*;

use moon_project_graph::ProjectGraph;
use moon_task_graph::TaskGraph;
use std::sync::Arc;

pub struct WorkspaceGraph {
    pub projects: Arc<ProjectGraph>,
    pub tasks: Arc<TaskGraph>,
}
