#![allow(dead_code)]

use moon_config::InheritedTasksConfig;
use std::path::Path;

pub struct TaskBuilder<'proj> {
    project_root: &'proj Path,
    workspace_root: &'proj Path,

    global_config: Option<&'proj InheritedTasksConfig>,
}

impl<'proj> TaskBuilder<'proj> {
    pub fn new(project_root: &'proj Path, workspace_root: &'proj Path) -> Self {
        Self {
            project_root,
            workspace_root,
            global_config: None,
        }
    }

    pub fn inherit_global_config(&mut self, config: &'proj InheritedTasksConfig) -> &mut Self {
        self.global_config = Some(config);
        self
    }
}
