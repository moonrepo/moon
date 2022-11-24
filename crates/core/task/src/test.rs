use crate::{FileGroup, Target, Task};
use moon_config::TaskConfig;
use moon_utils::string_vec;
use rustc_hash::FxHashMap;

pub fn create_file_groups_config() -> FxHashMap<String, Vec<String>> {
    let mut map = FxHashMap::default();

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

pub fn create_file_groups() -> FxHashMap<String, FileGroup> {
    let mut map = FxHashMap::default();

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
        Target::new("project", "task").unwrap(),
        &config.unwrap_or_default(),
    )
    .unwrap()
}
