use moon_task::FileGroup;
use moon_test_utils::get_fixtures_path;
use moon_utils::{glob, string_vec};
use std::path::PathBuf;

mod merge {
    use super::*;

    #[test]
    fn overwrites() {
        let mut file_group = FileGroup::new("id", string_vec!["**/*"]);

        file_group.merge(string_vec!["*"]);

        assert_eq!(file_group.files, string_vec!["*"]);
    }
}

mod all {
    use super::*;

    #[test]
    fn returns_all() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new(
            "id",
            string_vec!["**/*", "file.js", "folder/index.ts", "/root.js", "/root/*"],
        );

        assert_eq!(
            file_group.all(&workspace_root, &project_root).unwrap(),
            (
                vec![
                    project_root.join("file.js"),
                    project_root.join("folder/index.ts"),
                    workspace_root.join("root.js")
                ],
                vec![
                    glob::normalize(project_root.join("**/*")).unwrap(),
                    glob::normalize(workspace_root.join("root/*")).unwrap()
                ]
            )
        );
    }
}

mod dirs {
    use super::*;

    #[test]
    fn returns_all_dirs() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec!["**/*"]);

        assert_eq!(
            file_group.dirs(&workspace_root, &project_root).unwrap(),
            vec![project_root.join("dir"), project_root.join("dir/subdir")]
        );
    }

    #[test]
    fn doesnt_return_files() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec!["file.ts"]);
        let result: Vec<PathBuf> = vec![];

        assert_eq!(
            file_group.dirs(&workspace_root, &project_root).unwrap(),
            result
        );
    }
}

mod files {
    use super::*;

    #[test]
    fn returns_all_files() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new(
            "id",
            string_vec![
                // Globs
                "**/*.{ts,tsx}",
                "/*.json",
                // Literals
                "README.md",
                "/README.md"
            ],
        );

        let mut files = file_group.files(&workspace_root, &project_root).unwrap();
        files.sort();

        assert_eq!(
            files,
            vec![
                workspace_root.join("README.md"),
                project_root.join("README.md"),
                project_root.join("dir/other.tsx"),
                project_root.join("dir/subdir/another.ts"),
                project_root.join("file.ts"),
                workspace_root.join("package.json"),
            ]
        );
    }

    #[test]
    fn doesnt_return_dirs() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec!["dir"]);
        let result: Vec<PathBuf> = vec![];

        assert_eq!(
            file_group.files(&workspace_root, &project_root).unwrap(),
            result
        );
    }
}

mod globs {
    use super::*;

    #[test]
    fn returns_only_globs() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group =
            FileGroup::new("id", string_vec!["**/*", "*.rs", "file.ts", "dir", "/*.js"]);

        assert_eq!(
            file_group.globs(&workspace_root, &project_root).unwrap(),
            vec![
                glob::normalize(project_root.join("**/*")).unwrap(),
                glob::normalize(project_root.join("*.rs")).unwrap(),
                glob::normalize(workspace_root.join("*.js")).unwrap()
            ]
        );
    }
}

mod root {
    use super::*;

    #[test]
    fn returns_lowest_dir() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec!["**/*"]);

        assert_eq!(
            file_group.root(&project_root).unwrap(),
            project_root.join("dir")
        );
    }

    #[test]
    fn returns_root_when_many() {
        let workspace_root = get_fixtures_path("projects");
        let file_group = FileGroup::new("id", string_vec!["**/*"]);

        assert_eq!(file_group.root(&workspace_root).unwrap(), workspace_root);
    }

    #[test]
    fn returns_root_when_no_dirs() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec![]);

        assert_eq!(file_group.root(&project_root).unwrap(), project_root);
    }
}
