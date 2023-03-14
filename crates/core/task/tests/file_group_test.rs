use moon_task::FileGroup;
use moon_test_utils::get_fixtures_path;
use moon_utils::string_vec;
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

mod fg_all {
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
                    PathBuf::from("files-and-dirs").join("file.js"),
                    PathBuf::from("files-and-dirs").join("folder/index.ts"),
                    PathBuf::from("root.js")
                ],
                string_vec!["files-and-dirs/**/*", "root/*"]
            )
        );
    }
}

mod fg_dirs {
    use super::*;

    #[test]
    fn returns_all_dirs() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec!["**/*"]);

        assert_eq!(
            file_group.dirs(&workspace_root, &project_root).unwrap(),
            vec![
                PathBuf::from("files-and-dirs").join("dir"),
                PathBuf::from("files-and-dirs").join("dir/subdir")
            ]
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

mod fg_files {
    use super::*;

    #[test]
    fn returns_all_files() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let project_source = PathBuf::from("files-and-dirs");
        let file_group = FileGroup::new(
            "id",
            string_vec![
                // Globs
                "**/*.{ts,tsx}",
                "!dir/subdir/*",
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
                PathBuf::from("README.md"),
                project_source.join("README.md"),
                project_source.join("dir/other.tsx"),
                project_source.join("file.ts"),
                PathBuf::from("package.json"),
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

mod fg_globs {
    use super::*;

    #[test]
    fn returns_only_globs() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group =
            FileGroup::new("id", string_vec!["**/*", "*.rs", "file.ts", "dir", "/*.js"]);

        assert_eq!(
            file_group.globs(&workspace_root, &project_root).unwrap(),
            string_vec!["files-and-dirs/**/*", "files-and-dirs/*.rs", "*.js"]
        );
    }
}

mod fg_root {
    use super::*;

    #[test]
    fn returns_lowest_dir() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec!["**/*"]);

        assert_eq!(
            file_group.root(&workspace_root, &project_root).unwrap(),
            PathBuf::from("files-and-dirs").join("dir")
        );
    }

    #[test]
    fn returns_root_when_many() {
        let workspace_root = get_fixtures_path("projects");
        let file_group = FileGroup::new("id", string_vec!["**/*"]);

        assert_eq!(
            file_group.root(&workspace_root, &workspace_root).unwrap(),
            PathBuf::from(".")
        );
    }

    #[test]
    fn returns_root_when_no_dirs() {
        let workspace_root = get_fixtures_path("base");
        let project_root = workspace_root.join("files-and-dirs");
        let file_group = FileGroup::new("id", string_vec![]);

        assert_eq!(
            file_group.root(&workspace_root, &project_root).unwrap(),
            PathBuf::from(".")
        );
    }
}
