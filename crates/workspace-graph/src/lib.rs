use moon_project_graph::{Project, ProjectGraph};
use moon_task_graph::{Target, Task, TaskGraph};
use std::sync::Arc;

pub use moon_graph_utils::*;

pub struct WorkspaceGraph {
    pub projects: Arc<ProjectGraph>,
    pub tasks: Arc<TaskGraph>,
}

impl WorkspaceGraph {
    pub fn get_project(&self, id_or_alias: &str) -> miette::Result<Arc<Project>> {
        self.projects.get(id_or_alias)
    }

    pub fn get_project_with_tasks(&self, id_or_alias: &str) -> miette::Result<Project> {
        let base_project = self.get_project(id_or_alias)?;
        let mut project = base_project.as_ref().to_owned();

        for target in &base_project.task_targets {
            let base_task = self.get_task(target)?;

            project
                .tasks
                .insert(base_task.id.clone(), base_task.as_ref().to_owned());
        }

        Ok(project)
    }

    pub fn get_all_project(&self) -> miette::Result<Vec<Arc<Project>>> {
        self.projects.get_all()
    }

    pub fn get_task(&self, target: &Target) -> miette::Result<Arc<Task>> {
        self.tasks.get(target)
    }

    pub fn get_tasks_for_project(&self, project_id: &str) -> miette::Result<Vec<Arc<Task>>> {
        let project = self.get_project(project_id)?;
        let mut all = vec![];

        for target in &project.task_targets {
            let task = self.get_task(target)?;

            if !task.is_internal() {
                all.push(task);
            }
        }

        Ok(all)
    }

    pub fn get_all_tasks(&self) -> miette::Result<Vec<Arc<Task>>> {
        self.tasks.get_all()
    }
}
