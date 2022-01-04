use crate::file_group::FileGroup;
use std::collections::HashMap;
use std::path::Path;

pub fn create_file_groups_config() -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();

    map.insert(
        String::from("static"),
        vec![
            "file.ts".to_owned(),
            "dir".to_owned(),
            "dir/other.tsx".to_owned(),
            "dir/subdir".to_owned(),
            "dir/subdir/another.ts".to_owned(),
        ],
    );

    map.insert(String::from("dirs_glob"), vec!["**/*".to_owned()]);

    map.insert(String::from("files_glob"), vec!["**/*.{ts,tsx}".to_owned()]);

    map.insert(
        String::from("globs"),
        vec!["**/*.{ts,tsx}".to_owned(), "*.js".to_owned()],
    );

    map.insert(String::from("no_globs"), vec!["config.js".to_owned()]);

    map
}

pub fn create_file_groups(project_root: &Path) -> HashMap<String, FileGroup> {
    let mut map = HashMap::new();

    map.insert(
        String::from("static"),
        FileGroup::new(
            "static",
            vec![
                "file.ts".to_owned(),
                "dir".to_owned(),
                "dir/other.tsx".to_owned(),
                "dir/subdir".to_owned(),
                "dir/subdir/another.ts".to_owned(),
            ],
            project_root,
        ),
    );

    map.insert(
        String::from("dirs_glob"),
        FileGroup::new("dirs_glob", vec!["**/*".to_owned()], project_root),
    );

    map.insert(
        String::from("files_glob"),
        FileGroup::new("files_glob", vec!["**/*.{ts,tsx}".to_owned()], project_root),
    );

    map.insert(
        String::from("globs"),
        FileGroup::new(
            "globs",
            vec!["**/*.{ts,tsx}".to_owned(), "*.js".to_owned()],
            project_root,
        ),
    );

    map.insert(
        String::from("no_globs"),
        FileGroup::new("no_globs", vec!["config.js".to_owned()], project_root),
    );

    map
}
