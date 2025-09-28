#![allow(dead_code)]

use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{Input, TaskOptionCache};
use moon_graph_utils::GraphExpanderContext;
use moon_project::{FileGroup, Project};
use moon_task::{Target, Task, TaskFileInput, TaskFileOutput, TaskGlobInput, TaskGlobOutput};
use rustc_hash::FxHashMap;
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
                group.add(&Input::EnvVar("FOO_BAR".into()), "").unwrap();
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
            task.options.cache = TaskOptionCache::Enabled(false);
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

pub fn create_file_input_map(
    inputs: Vec<&str>,
) -> FxHashMap<WorkspaceRelativePathBuf, TaskFileInput> {
    FxHashMap::from_iter(
        inputs
            .into_iter()
            .map(|input| (input.into(), TaskFileInput::default())),
    )
}

pub fn create_glob_input_map(
    inputs: Vec<&str>,
) -> FxHashMap<WorkspaceRelativePathBuf, TaskGlobInput> {
    FxHashMap::from_iter(
        inputs
            .into_iter()
            .map(|input| (input.into(), TaskGlobInput::default())),
    )
}

pub fn create_file_output_map(
    outputs: Vec<&str>,
) -> FxHashMap<WorkspaceRelativePathBuf, TaskFileOutput> {
    FxHashMap::from_iter(
        outputs
            .into_iter()
            .map(|output| (output.into(), TaskFileOutput::default())),
    )
}

pub fn create_glob_output_map(
    outputs: Vec<&str>,
) -> FxHashMap<WorkspaceRelativePathBuf, TaskGlobOutput> {
    FxHashMap::from_iter(
        outputs
            .into_iter()
            .map(|output| (output.into(), TaskGlobOutput::default())),
    )
}
