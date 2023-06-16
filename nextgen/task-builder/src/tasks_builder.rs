#![allow(dead_code)]

use moon_config::{InheritedTasksConfig, InheritedTasksManager};
use std::path::Path;

pub struct TasksBuilder<'proj> {
    inheritance_manager: &'proj InheritedTasksManager,
    project_root: &'proj Path,
    workspace_root: &'proj Path,

    global_config: Option<InheritedTasksConfig>,
}

impl<'proj> TasksBuilder<'proj> {
    pub fn new(
        inheritance_manager: &'proj InheritedTasksManager,
        project_root: &'proj Path,
        workspace_root: &'proj Path,
    ) -> Self {
        Self {
            inheritance_manager,
            project_root,
            workspace_root,
            global_config: None,
        }
    }

    pub fn inherit_global_tasks(&mut self) {}
}
