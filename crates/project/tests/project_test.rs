use monolith_config::project::ProjectMetadataConfig;
use monolith_config::{FileGroups, GlobalProjectConfig, PackageJson, ProjectConfig};
use monolith_project::Project;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

fn get_fixture_root() -> PathBuf {
    let mut path = env::current_dir().unwrap();
    path.push("../../tests/fixtures");

    path
}

fn mock_file_groups() -> FileGroups {
    HashMap::from([(String::from("sources"), vec![String::from("src/**/*")])])
}

fn mock_global_project_config() -> GlobalProjectConfig {
    GlobalProjectConfig {
        file_groups: mock_file_groups(),
    }
}

#[test]
#[should_panic(expected = "DoesNotExist(\"projects/missing\")")]
fn doesnt_exist() {
    Project::new(
        "missing",
        "projects/missing",
        &get_fixture_root(),
        &mock_global_project_config(),
    )
    .unwrap();
}

#[test]
fn no_config() {
    let root_dir = get_fixture_root();
    let project = Project::new(
        "no-config",
        "projects/no-config",
        &root_dir,
        &mock_global_project_config(),
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("no-config"),
            config: None,
            dir: root_dir.join("projects/no-config").canonicalize().unwrap(),
            file_groups: mock_file_groups(),
            location: String::from("projects/no-config"),
            package_json: None,
        }
    );
}

#[test]
fn empty_config() {
    let root_dir = get_fixture_root();
    let project = Project::new(
        "empty-config",
        "projects/empty-config",
        &root_dir,
        &mock_global_project_config(),
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("empty-config"),
            config: Some(ProjectConfig {
                depends_on: None,
                file_groups: None,
                project: None,
            }),
            dir: root_dir
                .join("projects/empty-config")
                .canonicalize()
                .unwrap(),
            file_groups: mock_file_groups(),
            location: String::from("projects/empty-config"),
            package_json: None,
        }
    );
}

#[test]
fn basic_config() {
    let root_dir = get_fixture_root();
    let project = Project::new(
        "basic",
        "projects/basic",
        &root_dir,
        &mock_global_project_config(),
    )
    .unwrap();

    // Merges with global
    let mut file_groups = mock_file_groups();
    file_groups.insert(String::from("tests"), vec![String::from("**/*_test.rs")]);

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: Some(ProjectConfig {
                depends_on: Some(vec![String::from("other")]),
                file_groups: Some(HashMap::from([(
                    String::from("tests"),
                    vec![String::from("**/*_test.rs")]
                )])),
                project: None,
            }),
            dir: root_dir.join("projects/basic").canonicalize().unwrap(),
            file_groups,
            location: String::from("projects/basic"),
            package_json: None,
        }
    );
}

#[test]
fn advanced_config() {
    let root_dir = get_fixture_root();
    let project = Project::new(
        "advanced",
        "projects/advanced",
        &root_dir,
        &mock_global_project_config(),
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("advanced"),
            config: Some(ProjectConfig {
                depends_on: None,
                file_groups: None,
                project: Some(ProjectMetadataConfig {
                    name: String::from("Advanced"),
                    description: String::from("Advanced example."),
                    owner: String::from("Batman"),
                    maintainers: vec![String::from("Bruce Wayne")],
                    channel: String::from("#batcave"),
                }),
            }),
            dir: root_dir.join("projects/advanced").canonicalize().unwrap(),
            file_groups: mock_file_groups(),
            location: String::from("projects/advanced"),
            package_json: None,
        }
    );
}

#[test]
fn overrides_global_file_groups() {
    let root_dir = get_fixture_root();
    let project = Project::new(
        "basic",
        "projects/basic",
        &root_dir,
        &GlobalProjectConfig {
            file_groups: HashMap::from([(String::from("tests"), vec![String::from("tests/**/*")])]),
        },
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: Some(ProjectConfig {
                depends_on: Some(vec![String::from("other")]),
                file_groups: Some(HashMap::from([(
                    String::from("tests"),
                    vec![String::from("**/*_test.rs")]
                )])),
                project: None,
            }),
            dir: root_dir.join("projects/basic").canonicalize().unwrap(),
            file_groups: HashMap::from([(
                String::from("tests"),
                vec![String::from("**/*_test.rs")]
            )]),
            location: String::from("projects/basic"),
            package_json: None,
        }
    );
}

#[test]
fn has_package_json() {
    let root_dir = get_fixture_root();
    let project = Project::new(
        "package-json",
        "projects/package-json",
        &root_dir,
        &mock_global_project_config(),
    )
    .unwrap();

    let json = r#"
{
    "name": "npm-example",
    "version": "1.2.3",
    "scripts": {
        "build": "babel"
    }
}
"#;

    assert_eq!(
        project,
        Project {
            id: String::from("package-json"),
            config: None,
            dir: root_dir
                .join("projects/package-json")
                .canonicalize()
                .unwrap(),
            file_groups: mock_file_groups(),
            location: String::from("projects/package-json"),
            package_json: Some(PackageJson::from(json).unwrap()),
        }
    );
}
