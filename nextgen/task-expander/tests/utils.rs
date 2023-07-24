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

pub fn create_task() -> Task {
    Task {
        id: Id::raw("task"),
        target: Target::new("project", "task").unwrap(),
        ..Task::default()
    }
}
