use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_project::{FileGroup, Project};
use moon_task::{Target, Task};
use rustc_hash::FxHashMap;
use std::path::Path;

pub fn create_project(workspace_root: &Path) -> Project {
    let source = WorkspaceRelativePathBuf::from("project/source");

    Project {
        id: Id::raw("project"),
        root: workspace_root.join(source.as_str()),
        file_groups: FxHashMap::from_iter([
            (
                "all".into(),
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
                "dirs".into(),
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
        ]),
        source,
        ..Project::default()
    }
}

#[allow(dead_code)]
pub fn create_project_with_tasks(workspace_root: &Path, id: &str) -> Project {
    let mut project = create_project(workspace_root);
    project.id = Id::raw(id);

    for task_id in ["build", "lint", "test", "dev"] {
        let mut task = Task {
            id: Id::raw(task_id),
            target: Target::new(id, task_id).unwrap(),
            ..Task::default()
        };

        if task_id == "dev" {
            task.flags.local = true;
            task.options.cache = false;
            task.options.persistent = true;
        }

        project.tasks.insert(task_id.into(), task);
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
