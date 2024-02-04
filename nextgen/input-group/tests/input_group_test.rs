use moon_common::path::RelativePathBuf;
use moon_input_group::InputGroup;
use starbase_sandbox::locate_fixture;

fn file(path: &str) -> RelativePathBuf {
    RelativePathBuf::from("project").join(path)
}

#[test]
fn sets_patterns() {
    let input_group =
        InputGroup::new_with_source("id", [file("a"), file("*"), file("b"), file("**/*")]).unwrap();

    assert_eq!(
        input_group.files,
        vec![
            RelativePathBuf::from("project/a"),
            RelativePathBuf::from("project/b")
        ]
    );
    assert_eq!(
        input_group.globs,
        vec![
            RelativePathBuf::from("project/*"),
            RelativePathBuf::from("project/**/*")
        ]
    );
}

mod dirs {
    use super::*;

    #[test]
    fn returns_all_dirs() {
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source("id", [file("**/*")]).unwrap();

        assert_eq!(
            input_group.dirs(&workspace_root, false).unwrap(),
            vec![
                RelativePathBuf::from("project/dir"),
                RelativePathBuf::from("project/dir/subdir")
            ]
        );
    }

    #[test]
    fn doesnt_return_non_existent_dirs_nonloose_mode() {
        let workspace_root = locate_fixture("input-group");
        let input_group =
            InputGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")]).unwrap();

        assert!(input_group.dirs(&workspace_root, false).unwrap().is_empty());
    }

    #[test]
    fn returns_non_existent_dirs_loose_mode() {
        let workspace_root = locate_fixture("input-group");
        let input_group =
            InputGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")]).unwrap();

        assert_eq!(
            input_group.dirs(&workspace_root, true).unwrap(),
            vec![RelativePathBuf::from("project/fake/dir")]
        );
    }

    #[test]
    fn doesnt_return_files() {
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source("id", [file("**/*.json")]).unwrap();
        let result: Vec<RelativePathBuf> = vec![];

        assert_eq!(input_group.dirs(&workspace_root, false).unwrap(), result);
    }
}

mod files {
    use super::*;

    #[test]
    fn returns_project_files() {
        let workspace_root = locate_fixture("input-group");
        let input_group =
            InputGroup::new_with_source("id", [file("**/*.json"), file("docs.md")]).unwrap();

        let mut results = input_group.files(&workspace_root, false).unwrap();
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
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source(
            "id",
            [
                RelativePathBuf::from("*.json"),
                RelativePathBuf::from("docs.md"),
            ],
        )
        .unwrap();

        assert_eq!(
            input_group.files(&workspace_root, false).unwrap(),
            vec![
                RelativePathBuf::from("docs.md"),
                RelativePathBuf::from("workspace.json"),
            ]
        );
    }

    #[test]
    fn supports_negated_globs() {
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source(
            "id",
            [
                file("**/*.json"),
                RelativePathBuf::from("!project/dir/subdir/*"),
                file("docs.md"),
            ],
        )
        .unwrap();

        assert_eq!(
            input_group.files(&workspace_root, false).unwrap(),
            vec![
                RelativePathBuf::from("project/docs.md"),
                RelativePathBuf::from("project/project.json"),
            ]
        );
    }

    #[test]
    fn doesnt_return_non_existent_files_nonloose_mode() {
        let workspace_root = locate_fixture("input-group");
        let input_group =
            InputGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")]).unwrap();

        assert!(input_group
            .files(&workspace_root, false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn returns_non_existent_files_loose_mode() {
        let workspace_root = locate_fixture("input-group");
        let input_group =
            InputGroup::new_with_source("id", [file("fake/dir"), file("fake/file.txt")]).unwrap();

        assert_eq!(
            input_group.files(&workspace_root, true).unwrap(),
            vec![RelativePathBuf::from("project/fake/file.txt")]
        );
    }

    #[test]
    fn doesnt_return_dirs() {
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source("id", [file("dir")]).unwrap();
        let result: Vec<RelativePathBuf> = vec![];

        assert_eq!(input_group.files(&workspace_root, false).unwrap(), result);
    }
}

mod globs {
    use super::*;

    #[test]
    #[should_panic(expected = "No globs defined in input group id.")]
    fn errors_if_no_globs() {
        let input_group =
            InputGroup::new_with_source("id", [file("file.js"), file("docs.md")]).unwrap();

        input_group.globs().unwrap();
    }

    #[test]
    fn returns_only_globs() {
        let input_group = InputGroup::new_with_source(
            "id",
            [file("**/*.json"), file("file.js"), file("docs.md")],
        )
        .unwrap();

        assert_eq!(
            input_group.globs().unwrap(),
            &vec![RelativePathBuf::from("project/**/*.json")]
        );
    }
}

mod root {
    use super::*;

    #[test]
    fn returns_lowest_dir() {
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source("id", [file("**/*")]).unwrap();

        assert_eq!(
            input_group.root(workspace_root, "project").unwrap(),
            RelativePathBuf::from("project/dir")
        );
    }

    #[test]
    fn returns_root_when_many() {
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source("id", [file("**/*")]).unwrap();

        assert_eq!(
            input_group.root(workspace_root, ".").unwrap(),
            RelativePathBuf::from(".")
        );
    }

    #[test]
    fn returns_root_when_no_dirs() {
        let workspace_root = locate_fixture("input-group");
        let input_group = InputGroup::new_with_source("id", []).unwrap();

        assert_eq!(
            input_group.root(workspace_root, "project").unwrap(),
            RelativePathBuf::from(".")
        );
    }
}
