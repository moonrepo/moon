use moon_common::Id;
use moon_config::*;
use moon_task::Task;
use moon_task_builder::{TasksBuilder, TasksBuilderContext};
use moon_test_utils2::WorkspaceMocker;
use std::collections::BTreeMap;
use std::path::Path;

pub struct TasksBuilderContainer {
    pub mocker: WorkspaceMocker,
    pub monorepo: bool,
}

impl TasksBuilderContainer {
    pub fn new(fixture: &Path) -> Self {
        Self {
            mocker: WorkspaceMocker::new(fixture)
                .with_default_projects()
                .with_global_envs()
                .load_inherited_tasks_from("global"),
            monorepo: true,
        }
    }

    pub fn as_polyrepo(mut self) -> Self {
        self.monorepo = false;
        self
    }

    pub fn inherit_tasks_from(mut self, dir: &str) -> Self {
        self.mocker = self.mocker.load_inherited_tasks_from(dir);
        self
    }

    pub fn with_toolchains(mut self) -> Self {
        self.mocker = self.mocker.update_toolchain_config(|config| {
            config.bun = Some(BunConfig::default());
            config.deno = Some(DenoConfig::default());
            config.node = Some(NodeConfig::default());
            config.rust = Some(RustConfig::default());
            config.inherit_default_plugins().unwrap();
        });
        self
    }

    pub async fn build_tasks(&self, project_id: &str) -> BTreeMap<Id, Task> {
        let project = self.mocker.build_project(project_id).await;
        let toolchain_config = self.mocker.mock_toolchain_config();
        let toolchain_registry = self.mocker.mock_toolchain_registry();
        let enabled_toolchains = toolchain_config.get_enabled();

        let mut builder = TasksBuilder::new(
            &project.id,
            &project.source,
            &project.toolchains,
            TasksBuilderContext {
                enabled_toolchains: &enabled_toolchains,
                monorepo: self.monorepo,
                toolchain_config: &toolchain_config,
                toolchain_registry: toolchain_registry.into(),
                workspace_root: &self.mocker.workspace_root,
            },
        );

        builder.load_local_tasks(&project.config);

        let global_config = self
            .mocker
            .inherited_tasks
            .get_inherited_config(
                &project.toolchains,
                &project.config.stack,
                &project.config.type_of,
                &project.config.tags,
            )
            .unwrap();

        builder.inherit_global_tasks(
            &global_config.config,
            Some(&project.config.workspace.inherited_tasks),
        );

        builder.build().await.unwrap()
    }
}
