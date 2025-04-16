mod query_projects;
mod query_tasks;

use moon_common::Id;
use moon_project_graph::{Project, ProjectGraph};
use moon_task_graph::{Target, Task, TaskGraph};
use scc::HashMap;
use std::{path::Path, sync::Arc};

pub use moon_graph_utils::*;
pub use moon_project_graph as projects;
pub use moon_task_graph as tasks;

#[derive(Clone, Default)]
pub struct WorkspaceGraph {
    pub projects: Arc<ProjectGraph>,
    pub tasks: Arc<TaskGraph>,

    /// Cache of query results, mapped by query input to project IDs.
    project_query_cache: HashMap<String, Arc<Vec<Id>>>,

    /// Cache of query results, mapped by query input to task targets.
    task_query_cache: HashMap<String, Arc<Vec<Target>>>,
}

impl WorkspaceGraph {
    pub fn new(projects: Arc<ProjectGraph>, tasks: Arc<TaskGraph>) -> Self {
        Self {
            projects,
            tasks,
            project_query_cache: HashMap::default(),
            task_query_cache: HashMap::default(),
        }
    }

    pub fn get_project(&self, id_or_alias: impl AsRef<str>) -> miette::Result<Arc<Project>> {
        self.projects.get(id_or_alias.as_ref())
    }

    pub fn get_project_from_path(
        &self,
        starting_file: Option<&Path>,
    ) -> miette::Result<Arc<Project>> {
        self.projects.get_from_path(starting_file)
    }

    pub fn get_project_with_tasks(&self, id_or_alias: impl AsRef<str>) -> miette::Result<Project> {
        let base_project = self.get_project(id_or_alias)?;
        let mut project = base_project.as_ref().to_owned();

        for base_task in self.get_tasks_from_project(&project.id)? {
            project
                .tasks
                .insert(base_task.id.clone(), base_task.as_ref().to_owned());
        }

        Ok(project)
    }

    pub fn get_projects(&self) -> miette::Result<Vec<Arc<Project>>> {
        self.projects.get_all()
    }

    pub fn get_task(&self, target: &Target) -> miette::Result<Arc<Task>> {
        self.tasks.get(target)
    }

    pub fn get_task_from_project(
        &self,
        project_id_or_alias: impl AsRef<str>,
        task_id: impl AsRef<str>,
    ) -> miette::Result<Arc<Task>> {
        let project_id = self.projects.resolve_id(project_id_or_alias.as_ref());
        let target = Target::new(project_id, task_id)?;

        self.tasks.get(&target)
    }

    pub fn get_tasks_from_project(
        &self,
        project_id_or_alias: impl AsRef<str>,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let project = self.get_project(project_id_or_alias)?;
        let mut all = vec![];

        for target in &project.task_targets {
            let task = self.get_task(target)?;

            if !task.is_internal() {
                all.push(task);
            }
        }

        Ok(all)
    }

    /// Get all non-internal tasks.
    pub fn get_tasks(&self) -> miette::Result<Vec<Arc<Task>>> {
        Ok(self
            .tasks
            .get_all()?
            .into_iter()
            .filter(|task| !task.is_internal())
            .collect())
    }

    /// Get all tasks, including internal.
    pub fn get_tasks_with_internal(&self) -> miette::Result<Vec<Arc<Task>>> {
        self.tasks.get_all()
    }
}
