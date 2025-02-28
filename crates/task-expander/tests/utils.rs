#![allow(dead_code)]

use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::InputPath;
use moon_graph_utils::GraphExpanderContext;
use moon_project::{FileGroup, Project};
use moon_task::{Target, Task};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub fn create_context(workspace_root: &Path) -> GraphExpanderContext {
    GraphExpanderContext {
        vcs_branch: Arc::new(String::from("master")),
        vcs_repository: Arc::new(String::from("moonrepo/moon")),
        vcs_revision: Arc::new(String::from("abcd1234")),
        working_dir: workspace_root.to_path_buf(),
        workspace_root: workspace_root.to_path_buf(),
    }
}

pub fn create_project(workspace_root: &Path) -> Project {
    let source = WorkspaceRelativePathBuf::from("project/source");

    let mut project = Project {
        id: Id::raw("project"),
        root: workspace_root.join(source.as_str()),
        file_groups: BTreeMap::from_iter([
            (
                Id::raw("all"),
                FileGroup::new_with_source(
                    "all",
                    [
                        source.join("*.md"),
                        source.join("**/*.json"),
                        source.join("config.yml"),
                        source.join("dir/subdir"),
                    ],
                )
                .unwrap(),
            ),
            (
                Id::raw("dirs"),
                FileGroup::new_with_source(
                    "dirs",
                    [
                        source.join("other"),
                        source.join("dir/*"),
                        source.join("**/*.md"),
                    ],
                )
                .unwrap(),
            ),
            (Id::raw("envs"), {
                let mut group = FileGroup::new("envs").unwrap();
                group.add(&InputPath::EnvVar("FOO_BAR".into()), "").unwrap();
                group
            }),
        ]),
        source,
        ..Project::default()
    };

    project.tasks.insert(Id::raw("task"), create_task());
    project
}

pub fn create_project_with_tasks(workspace_root: &Path, id: &str) -> Project {
    let mut project = create_project(workspace_root);
    project.id = Id::raw(id);

    for task_id in ["build", "lint", "test", "test-fail", "dev"] {
        let mut task = Task {
            id: Id::raw(task_id),
            target: Target::new(id, task_id).unwrap(),
            ..Task::default()
        };

        if task_id == "dev" {
            task.state.local_only = true;
            task.options.cache = false;
            task.options.persistent = true;
        }

        if task_id == "test-fail" {
            task.options.allow_failure = true;
        }

        project.tasks.insert(task.id.clone(), task);
    }

    project
}

pub fn create_task() -> Task {
    Task {
        id: Id::raw("task"),
        target: Target::new("project", "task").unwrap(),
        ..Task::default()
    }
}
