use moon_common::path::{normalize_separators, WorkspaceRelativePathBuf};
use moon_common::Id;
use moon_config::{
    InheritedTasksManager, LanguageType, PartialInheritedTasksConfig, ProjectConfig,
    ProjectDependsOn, ProjectMetadataConfig, ProjectType,
};
use moon_file_group::FileGroup;
use moon_project::Project;
use moon_test_utils::{create_input_paths, get_fixtures_root};
use rustc_hash::FxHashMap;

fn mock_file_groups(source: &str) -> FxHashMap<Id, FileGroup> {
    FxHashMap::from_iter([(
        "sources".into(),
        FileGroup::new_with_source(
            "sources",
            [WorkspaceRelativePathBuf::from(format!("{source}/src/**/*"))],
        )
        .unwrap(),
    )])
}

fn mock_tasks_config() -> InheritedTasksManager {
    let config = PartialInheritedTasksConfig {
        file_groups: Some(FxHashMap::from_iter([(
            "sources".into(),
            create_input_paths(["src/**/*"]),
        )])),
        ..PartialInheritedTasksConfig::default()
    };

    let mut manager = InheritedTasksManager::default();
    manager.configs.insert("*".into(), config);

    manager
}

#[test]
#[should_panic(expected = "MissingProjectAtSource")]
fn doesnt_exist() {
    Project::new(
        &Id::raw("missing"),
        "projects/missing",
        &get_fixtures_root(),
        &mock_tasks_config(),
        |_| LanguageType::Unknown,
    )
    .unwrap();
}

#[test]
fn no_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        &Id::raw("no-config"),
        "projects/no-config",
        &workspace_root,
        &mock_tasks_config(),
        |_| LanguageType::Unknown,
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: "no-config".into(),
            log_target: "moon:project:no-config".into(),
            root: workspace_root.join(normalize_separators("projects/no-config")),
            file_groups: mock_file_groups("projects/no-config"),
            source: WorkspaceRelativePathBuf::from("projects/no-config"),
            ..Project::default()
        }
    );
}

#[test]
fn empty_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        &Id::raw("empty-config"),
        "projects/empty-config",
        &workspace_root,
        &mock_tasks_config(),
        |_| LanguageType::Unknown,
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: "empty-config".into(),
            config: ProjectConfig::default(),
            log_target: "moon:project:empty-config".into(),
            root: workspace_root.join(normalize_separators("projects/empty-config")),
            file_groups: mock_file_groups("projects/empty-config"),
            source: WorkspaceRelativePathBuf::from("projects/empty-config"),
            ..Project::default()
        }
    );
}

#[test]
fn basic_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        &Id::raw("basic"),
        "projects/basic",
        &workspace_root,
        &mock_tasks_config(),
        |_| LanguageType::Unknown,
    )
    .unwrap();
    let project_root = workspace_root.join(normalize_separators("projects/basic"));

    // Merges with global
    let mut file_groups = mock_file_groups("projects/basic");
    file_groups.insert(
        "tests".into(),
        FileGroup::new_with_source(
            "tests",
            [WorkspaceRelativePathBuf::from(
                "projects/basic/**/*_test.rs",
            )],
        )
        .unwrap(),
    );

    assert_eq!(
        project,
        Project {
            id: "basic".into(),
            config: ProjectConfig {
                depends_on: vec![ProjectDependsOn::String("noConfig".into())],
                file_groups: FxHashMap::from_iter([(
                    "tests".into(),
                    create_input_paths(["**/*_test.rs"])
                )]),
                language: LanguageType::JavaScript,
                tags: vec![Id::raw("vue")],
                ..ProjectConfig::default()
            },
            log_target: "moon:project:basic".into(),
            root: project_root,
            file_groups,
            source: WorkspaceRelativePathBuf::from("projects/basic"),
            ..Project::default()
        }
    );
}

#[test]
fn advanced_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        &Id::raw("advanced"),
        "projects/advanced",
        &workspace_root,
        &mock_tasks_config(),
        |_| LanguageType::Unknown,
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
                    maintainers: vec!["Bruce Wayne".into()],
                    channel: Some("#batcave".into()),
                }),
                tags: vec![Id::raw("react")],
                type_of: ProjectType::Application,
                language: LanguageType::TypeScript,
                ..ProjectConfig::default()
            },
            log_target: "moon:project:advanced".into(),
            root: workspace_root.join(normalize_separators("projects/advanced")),
            file_groups: mock_file_groups("projects/advanced"),
            source: WorkspaceRelativePathBuf::from("projects/advanced"),
            ..Project::default()
        }
    );
}
