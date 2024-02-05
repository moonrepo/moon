use moon_common::path::RelativePathBuf;
use moon_config::InputPath;
use moon_file_group::FileGroup;
use starbase_sandbox::locate_fixture;

fn file(path: &str) -> RelativePathBuf {
    RelativePathBuf::from("project").join(path)
}

mod file_group {
    use super::*;

    #[test]
    fn sets_patterns() {
        let file_group =
            FileGroup::new_with_source("id", [file("a"), file("*"), file("b"), file("**/*")])
                .unwrap();

        assert_eq!(
            file_group.files,
            vec![
                RelativePathBuf::from("project/a"),
                RelativePathBuf::from("project/b")
            ]
        );
        assert_eq!(
            file_group.globs,
            vec![
                RelativePathBuf::from("project/*"),
                RelativePathBuf::from("project/**/*")
            ]
        );
    }

    #[test]
    #[should_panic(expected = "Token functions and variables are not supported in file groups.")]
    fn errors_for_token_funcs() {
        let mut file_group = FileGroup::new("id").unwrap();

        file_group
            .add(&InputPath::TokenFunc("@group(name)".into()), "")
            .unwrap();

        assert_eq!(file_group.env, ["FOO_BAR"]);
    }

    #[test]
    #[should_panic(expected = "Token functions and variables are not supported in file groups.")]
    fn errors_for_token_vars() {
        let mut file_group = FileGroup::new("id").unwrap();

        file_group
            .add(&InputPath::TokenVar("$var".into()), "")
            .unwrap();

        assert_eq!(file_group.env, ["FOO_BAR"]);
    }

    #[test]
    fn can_set_env_vars() {
        let mut file_group = FileGroup::new("id").unwrap();

        file_group
            .add(&InputPath::EnvVar("FOO_BAR".into()), "")
            .unwrap();

        assert_eq!(file_group.env, ["FOO_BAR"]);
    }

    #[test]
    fn can_set_file_paths() {
        let mut file_group = FileGroup::new("id").unwrap();

        file_group
            .add(&InputPath::ProjectFile("file.js".into()), "test-source")
            .unwrap();

        file_group
            .add(&InputPath::WorkspaceFile("root.js".into()), "test-source")
            .unwrap();

        assert_eq!(file_group.files, ["test-source/file.js", "root.js"]);
    }

    #[test]
    fn can_set_globs() {
        let mut file_group = FileGroup::new("id").unwrap();

        file_group
            .add(&InputPath::ProjectGlob("*.js".into()), "test-source")
            .unwrap();

        file_group
            .add(&InputPath::WorkspaceGlob("root.*".into()), "test-source")
            .unwrap();

        assert_eq!(file_group.globs, ["test-source/*.js", "root.*"]);
    }

    mod dirs {
        use super::*;

        #[test]
        fn returns_all_dirs() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source("id", [file("**/*")]).unwrap();

            assert_eq!(
                file_group.dirs(&workspace_root, false).unwrap(),
                vec![
                    RelativePathBuf::from("project/dir"),
                    RelativePathBuf::from("project/dir/subdir")
                ]
            );
        }

        #[test]
        fn doesnt_return_non_existent_dirs_nonloose_mode() {
            let workspace_root = locate_fixture("file-group");
            let file_group =
                FileGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")])
                    .unwrap();

            assert!(file_group.dirs(&workspace_root, false).unwrap().is_empty());
        }

        #[test]
        fn returns_non_existent_dirs_loose_mode() {
            let workspace_root = locate_fixture("file-group");
            let file_group =
                FileGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")])
                    .unwrap();

            assert_eq!(
                file_group.dirs(&workspace_root, true).unwrap(),
                vec![RelativePathBuf::from("project/fake/dir")]
            );
        }

        #[test]
        fn doesnt_return_files() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source("id", [file("**/*.json")]).unwrap();
            let result: Vec<RelativePathBuf> = vec![];

            assert_eq!(file_group.dirs(&workspace_root, false).unwrap(), result);
        }
    }

    mod files {
        use super::*;

        #[test]
        fn returns_project_files() {
            let workspace_root = locate_fixture("file-group");
            let file_group =
                FileGroup::new_with_source("id", [file("**/*.json"), file("docs.md")]).unwrap();

            let mut results = file_group.files(&workspace_root, false).unwrap();
            results.sort();

            assert_eq!(
                results,
                vec![
                    RelativePathBuf::from("project/dir/subdir/nested.json"),
                    RelativePathBuf::from("project/docs.md"),
                    RelativePathBuf::from("project/project.json"),
                ]
            );
        }

        #[test]
        fn returns_workspace_files() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source(
                "id",
                [
                    RelativePathBuf::from("*.json"),
                    RelativePathBuf::from("docs.md"),
                ],
            )
            .unwrap();

            assert_eq!(
                file_group.files(&workspace_root, false).unwrap(),
                vec![
                    RelativePathBuf::from("docs.md"),
                    RelativePathBuf::from("workspace.json"),
                ]
            );
        }

        #[test]
        fn supports_negated_globs() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source(
                "id",
                [
                    file("**/*.json"),
                    RelativePathBuf::from("!project/dir/subdir/*"),
                    file("docs.md"),
                ],
            )
            .unwrap();

            assert_eq!(
                file_group.files(&workspace_root, false).unwrap(),
                vec![
                    RelativePathBuf::from("project/docs.md"),
                    RelativePathBuf::from("project/project.json"),
                ]
            );
        }

        #[test]
        fn doesnt_return_non_existent_files_nonloose_mode() {
            let workspace_root = locate_fixture("file-group");
            let file_group =
                FileGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")])
                    .unwrap();

            assert!(file_group.files(&workspace_root, false).unwrap().is_empty());
        }

        #[test]
        fn returns_non_existent_files_loose_mode() {
            let workspace_root = locate_fixture("file-group");
            let file_group =
                FileGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")])
                    .unwrap();

            assert_eq!(
                file_group.files(&workspace_root, true).unwrap(),
                vec![RelativePathBuf::from("project/fake/file.txt")]
            );
        }

        #[test]
        fn doesnt_return_dirs() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source("id", [file("dir")]).unwrap();
            let result: Vec<RelativePathBuf> = vec![];

            assert_eq!(file_group.files(&workspace_root, false).unwrap(), result);
        }
    }

    mod globs {
        use super::*;

        #[test]
        #[should_panic(expected = "No globs defined in file group id.")]
        fn errors_if_no_globs() {
            let file_group =
                FileGroup::new_with_source("id", [file("file.js"), file("docs.md")]).unwrap();

            file_group.globs().unwrap();
        }

        #[test]
        fn returns_only_globs() {
            let file_group = FileGroup::new_with_source(
                "id",
                [file("**/*.json"), file("file.js"), file("docs.md")],
            )
            .unwrap();

            assert_eq!(
                file_group.globs().unwrap(),
                &vec![RelativePathBuf::from("project/**/*.json")]
            );
        }
    }

    mod root {
        use super::*;

        #[test]
        fn returns_lowest_dir() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source("id", [file("**/*")]).unwrap();

            assert_eq!(
                file_group.root(workspace_root, "project").unwrap(),
                RelativePathBuf::from("project/dir")
            );
        }

        #[test]
        fn returns_root_when_many() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source("id", [file("**/*")]).unwrap();

            assert_eq!(
                file_group.root(workspace_root, ".").unwrap(),
                RelativePathBuf::from(".")
            );
        }

        #[test]
        fn returns_root_when_no_dirs() {
            let workspace_root = locate_fixture("file-group");
            let file_group = FileGroup::new_with_source("id", []).unwrap();

            assert_eq!(
                file_group.root(workspace_root, "project").unwrap(),
                RelativePathBuf::from(".")
            );
        }
    }
}
