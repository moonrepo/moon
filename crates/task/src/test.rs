use crate::{FileGroup, Target, Task, TaskError, TokenResolver, TokenSharedData};
use moon_config::{ProjectConfig, TaskConfig};
use moon_utils::string_vec;
use std::collections::HashMap;
use std::path::Path;

pub fn create_file_groups_config() -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();

    map.insert(
        String::from("static"),
        string_vec![
            "file.ts",
            "dir",
            "dir/other.tsx",
            "dir/subdir",
            "dir/subdir/another.ts",
        ],
    );

    map.insert(String::from("dirs_glob"), string_vec!["**/*"]);

    map.insert(String::from("files_glob"), string_vec!["**/*.{ts,tsx}"]);

    map.insert(String::from("globs"), string_vec!["**/*.{ts,tsx}", "*.js"]);

    map.insert(String::from("no_globs"), string_vec!["config.js"]);

    map
}

pub fn create_file_groups() -> HashMap<String, FileGroup> {
    let mut map = HashMap::new();

    map.insert(
        String::from("static"),
        FileGroup::new(
            "static",
            string_vec![
                "file.ts",
                "dir",
                "dir/other.tsx",
                "dir/subdir",
                "dir/subdir/another.ts",
            ],
        ),
    );

    map.insert(
        String::from("dirs_glob"),
        FileGroup::new("dirs_glob", string_vec!["**/*"]),
    );

    map.insert(
        String::from("files_glob"),
        FileGroup::new("files_glob", string_vec!["**/*.{ts,tsx}"]),
    );

    map.insert(
        String::from("globs"),
        FileGroup::new("globs", string_vec!["**/*.{ts,tsx}", "*.js"]),
    );

    map.insert(
        String::from("no_globs"),
        FileGroup::new("no_globs", string_vec!["config.js"]),
    );

    map
}

pub fn create_initial_task(config: Option<TaskConfig>) -> Task {
    Task::from_config(
        Target::format("project", "task").unwrap(),
        &config.unwrap_or_default(),
    )
}

pub fn create_expanded_task(
    workspace_root: &Path,
    project_root: &Path,
    config: Option<TaskConfig>,
) -> Result<Task, TaskError> {
    let mut task = create_initial_task(config);
    let file_groups = create_file_groups();
    let project_config = ProjectConfig::new(project_root);
    let metadata =
        TokenSharedData::new(&file_groups, workspace_root, project_root, &project_config);

    task.expand_env(&metadata)?;
    task.expand_inputs(TokenResolver::for_inputs(&metadata))?;
    task.expand_outputs(TokenResolver::for_outputs(&metadata))?;
    task.expand_args(TokenResolver::for_args(&metadata))?; // Must be last

    Ok(task)
}
