use moon_config::TaskConfig;
use moon_task::test::create_expanded_task;
use moon_utils::string_vec;
use moon_utils::test::get_fixtures_dir;
use std::collections::HashSet;

#[test]
#[should_panic(expected = "NoOutputGlob")]
fn errors_for_output_glob() {
    let workspace_root = get_fixtures_dir("projects");
    let project_root = workspace_root.join("basic");

    create_expanded_task(
        &workspace_root,
        &project_root,
        Some(TaskConfig {
            outputs: Some(string_vec!["some/**/glob"]),
            ..TaskConfig::default()
        }),
    )
    .unwrap();
}

mod is_affected {
    use super::*;

    #[test]
    fn returns_true_if_matches_file() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = HashSet::new();
        set.insert(project_root.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_if_matches_glob() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.*"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = HashSet::new();
        set.insert(project_root.join("file.ts"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_true_when_referencing_root_files() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["/package.json"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = HashSet::new();
        set.insert(workspace_root.join("package.json"));

        assert!(task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_outside_project() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = HashSet::new();
        set.insert(workspace_root.join("base/other/outside.ts"));

        assert!(!task.is_affected(&set).unwrap());
    }

    #[test]
    fn returns_false_if_no_match() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["file.ts", "src/*"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        let mut set = HashSet::new();
        set.insert(project_root.join("another.rs"));

        assert!(!task.is_affected(&set).unwrap());
    }
}
