#![allow(dead_code)]

use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_project::{FileGroup, Project};
use moon_project_expander::ExpanderContext;
use moon_task::{Target, Task};
use rustc_hash::FxHashMap;
use std::path::Path;

pub fn create_context<'g, 'q>(
    project: &'g Project,
    workspace_root: &'g Path,
) -> ExpanderContext<'g, 'q> {
    ExpanderContext {
        aliases: FxHashMap::default(),
        check_boundaries: true,
        project,
        query: Box::new(|_| Ok(vec![])),
        workspace_root,
    }
}

pub fn create_context_with_query<'g, 'q, Q>(
    project: &'g Project,
    workspace_root: &'g Path,
    query: Q,
) -> ExpanderContext<'g, 'q>
where
    Q: Fn(String) -> miette::Result<Vec<&'q Project>> + 'g,
{
    let mut context = create_context(project, workspace_root);
    context.query = Box::new(query);
    context
}

pub fn create_project(workspace_root: &Path) -> Project {
    let source = WorkspaceRelativePathBuf::from("project/source");

    let mut project = Project {
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
    };

    project.tasks.insert("task".into(), create_task());
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
            task.flags.local = true;
            task.options.cache = false;
            task.options.persistent = true;
        }

        if task_id == "test-fail" {
            task.options.allow_failure = true;
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
