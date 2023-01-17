use moon_config::{
    GlobalProjectConfig, ProjectConfig, ProjectDependsOn, ProjectLanguage, ProjectMetadataConfig,
    ProjectType,
};
use moon_project::Project;
use moon_task::FileGroup;
use moon_test_utils::get_fixtures_root;
use moon_utils::string_vec;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

fn mock_file_groups() -> FxHashMap<String, FileGroup> {
    FxHashMap::from_iter([(
        "sources".into(),
        FileGroup::new("sources", string_vec!["src/**/*"]),
    )])
}

fn mock_tasks_config() -> GlobalProjectConfig {
    GlobalProjectConfig {
        extends: None,
        file_groups: FxHashMap::from_iter([("sources".into(), string_vec!["src/**/*"])]),
        tasks: BTreeMap::new(),
        schema: String::new(),
    }
}

#[test]
#[should_panic(expected = "MissingProjectAtSource(\"projects/missing\")")]
fn doesnt_exist() {
    Project::new(
        "missing",
        "projects/missing",
        &get_fixtures_root(),
        &mock_tasks_config(),
    )
    .unwrap();
}

#[test]
fn no_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "no-config",
        "projects/no-config",
        &workspace_root,
        &mock_tasks_config(),
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: "no-config".into(),
            log_target: "moon:project:no-config".into(),
            root: workspace_root.join("projects/no-config"),
            file_groups: mock_file_groups(),
            source: "projects/no-config".into(),
            ..Project::default()
        }
    );
}

#[test]
fn empty_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "empty-config",
        "projects/empty-config",
        &workspace_root,
        &mock_tasks_config(),
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: "empty-config".into(),
            config: ProjectConfig::default(),
            log_target: "moon:project:empty-config".into(),
            root: workspace_root.join("projects/empty-config"),
            file_groups: mock_file_groups(),
            source: "projects/empty-config".into(),
            ..Project::default()
        }
    );
}

#[test]
fn basic_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "basic",
        "projects/basic",
        &workspace_root,
        &mock_tasks_config(),
    )
    .unwrap();
    let project_root = workspace_root.join("projects/basic");

    // Merges with global
    let mut file_groups = mock_file_groups();
    file_groups.insert(
        "tests".into(),
        FileGroup::new("tests", string_vec!["**/*_test.rs"]),
    );

    assert_eq!(
        project,
        Project {
            id: "basic".into(),
            config: ProjectConfig {
                depends_on: vec![ProjectDependsOn::String("noConfig".to_owned())],
                file_groups: FxHashMap::from_iter([("tests".into(), string_vec!["**/*_test.rs"])]),
                language: ProjectLanguage::JavaScript,
                ..ProjectConfig::default()
            },
            log_target: "moon:project:basic".into(),
            root: project_root,
            file_groups,
            source: "projects/basic".into(),
            ..Project::default()
        }
    );
}

#[test]
fn advanced_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "advanced",
        "projects/advanced",
        &workspace_root,
        &mock_tasks_config(),
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: "advanced".into(),
            config: ProjectConfig {
                project: Some(ProjectMetadataConfig {
                    name: Some("Advanced".into()),
                    description: "Advanced example.".into(),
                    owner: Some("Batman".into()),
                    maintainers: Some(string_vec!["Bruce Wayne"]),
                    channel: Some("#batcave".into()),
                }),
                type_of: ProjectType::Application,
                language: ProjectLanguage::TypeScript,
                ..ProjectConfig::default()
            },
            log_target: "moon:project:advanced".into(),
            root: workspace_root.join("projects/advanced"),
            file_groups: mock_file_groups(),
            source: "projects/advanced".into(),
            ..Project::default()
        }
    );
}
