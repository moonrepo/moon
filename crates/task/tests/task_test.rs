use moon_config::TaskConfig;
use moon_task::test::create_expanded_task;
use moon_utils::test::get_fixtures_dir;
use moon_utils::{glob, string_vec};
use std::collections::HashSet;
use std::env;

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
    fn returns_true_if_var_truthy() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["$FOO"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        env::set_var("FOO", "foo");

        assert!(task.is_affected(&HashSet::new()).unwrap());

        env::remove_var("FOO");
    }
    #[test]
    fn returns_false_if_var_missing() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["$BAR"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert!(!task.is_affected(&HashSet::new()).unwrap());
    }

    #[test]
    fn returns_false_if_var_empty() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["$BAZ"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        env::set_var("BAZ", "");

        assert!(!task.is_affected(&HashSet::new()).unwrap());

        env::remove_var("BAZ");
    }

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

mod expand_inputs {
    use super::*;

    #[test]
    fn filters_into_correct_types() {
        let workspace_root = get_fixtures_dir("base");
        let project_root = workspace_root.join("files-and-dirs");
        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec![
                    "$VAR",
                    "$FOO_BAR",
                    "file.ts",
                    "folder",
                    "glob/**/*",
                    "/config.js",
                    "/*.cfg"
                ]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            task.input_vars,
            HashSet::from(["VAR".to_owned(), "FOO_BAR".to_owned()])
        );
        assert_eq!(
            task.input_paths,
            HashSet::from([
                project_root.join("file.ts"),
                project_root.join("folder"),
                workspace_root.join("config.js")
            ])
        );
        assert_eq!(
            task.input_globs,
            HashSet::from([
                glob::normalize(project_root.join("glob/**/*")).unwrap(),
                glob::normalize(workspace_root.join("*.cfg")).unwrap()
            ])
        );
    }
}
