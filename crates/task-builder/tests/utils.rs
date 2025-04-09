use moon_common::Id;
use moon_task::Task;
use moon_test_utils2::WorkspaceMocker;
use std::collections::BTreeMap;
use std::path::Path;

pub struct TasksBuilderContainer {
    pub mocker: WorkspaceMocker,
}

impl TasksBuilderContainer {
    pub fn new(root: &Path) -> Self {
        Self {
            mocker: WorkspaceMocker::new(root)
                .with_default_projects()
                .with_global_envs()
                .load_inherited_tasks_from("global"),
        }
    }

    pub fn with_all_toolchains(mut self) -> Self {
        self.mocker = self.mocker.with_all_toolchains();
        self
    }

    pub fn with_global_tasks(mut self, dir: &str) -> Self {
        self.mocker = self.mocker.load_inherited_tasks_from(dir);
        self
    }

    pub fn with_polyrepo(mut self) -> Self {
        self.mocker = self.mocker.set_polyrepo();
        self
    }

    pub async fn build_tasks(&self, project_id: &str) -> BTreeMap<Id, Task> {
        let project = self.mocker.build_project(project_id).await;
        self.mocker.build_tasks(&project).await
    }
}
