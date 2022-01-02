use moon_config::{
    FileGroups, GlobalProjectConfig, PackageJson, ProjectConfig, ProjectMetadataConfig,
    ProjectType, TargetID, TaskConfig, TaskMergeStrategy, TaskOptionsConfig, TaskType,
};
use moon_project::{Project, ProjectError, Target, Task};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

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
        file_groups: Some(mock_file_groups()),
        tasks: None,
    }
}

#[test]
#[should_panic(expected = "MissingFilePath(\"projects/missing\")")]
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
            tasks: HashMap::new(),
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
                tasks: None,
            }),
            dir: root_dir
                .join("projects/empty-config")
                .canonicalize()
                .unwrap(),
            file_groups: mock_file_groups(),
            location: String::from("projects/empty-config"),
            package_json: None,
            tasks: HashMap::new(),
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
                depends_on: Some(vec![String::from("noConfig")]),
                file_groups: Some(HashMap::from([(
                    String::from("tests"),
                    vec![String::from("**/*_test.rs")]
                )])),
                project: None,
                tasks: None,
            }),
            dir: root_dir.join("projects/basic").canonicalize().unwrap(),
            file_groups,
            location: String::from("projects/basic"),
            package_json: None,
            tasks: HashMap::new(),
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
                    type_of: ProjectType::Library,
                    name: String::from("Advanced"),
                    description: String::from("Advanced example."),
                    owner: String::from("Batman"),
                    maintainers: vec![String::from("Bruce Wayne")],
                    channel: String::from("#batcave"),
                }),
                tasks: None,
            }),
            dir: root_dir.join("projects/advanced").canonicalize().unwrap(),
            file_groups: mock_file_groups(),
            location: String::from("projects/advanced"),
            package_json: None,
            tasks: HashMap::new(),
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
            file_groups: Some(HashMap::from([(
                String::from("tests"),
                vec![String::from("tests/**/*")],
            )])),
            tasks: None,
        },
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: Some(ProjectConfig {
                depends_on: Some(vec![String::from("noConfig")]),
                file_groups: Some(HashMap::from([(
                    String::from("tests"),
                    vec![String::from("**/*_test.rs")]
                )])),
                project: None,
                tasks: None,
            }),
            dir: root_dir.join("projects/basic").canonicalize().unwrap(),
            file_groups: HashMap::from([(
                String::from("tests"),
                vec![String::from("**/*_test.rs")]
            )]),
            location: String::from("projects/basic"),
            package_json: None,
            tasks: HashMap::new(),
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
            tasks: HashMap::new(),
        }
    );
}

mod tasks {
    use super::*;

    fn mock_task_config(command: &str) -> TaskConfig {
        TaskConfig {
            args: None,
            command: Some(command.to_owned()),
            deps: None,
            inputs: None,
            outputs: None,
            options: None,
            type_of: None,
        }
    }

    fn mock_merged_task_options_config(strategy: TaskMergeStrategy) -> TaskOptionsConfig {
        TaskOptionsConfig {
            merge_args: Some(strategy.clone()),
            merge_deps: Some(strategy.clone()),
            merge_inputs: Some(strategy.clone()),
            merge_outputs: Some(strategy),
            retry_count: Some(1),
            run_in_ci: Some(true),
            run_from_workspace_root: None,
        }
    }

    fn mock_local_task_options_config(strategy: TaskMergeStrategy) -> TaskOptionsConfig {
        TaskOptionsConfig {
            merge_args: Some(strategy.clone()),
            merge_deps: Some(strategy.clone()),
            merge_inputs: Some(strategy.clone()),
            merge_outputs: Some(strategy),
            retry_count: None,
            run_in_ci: None,
            run_from_workspace_root: None,
        }
    }

    fn stub_global_task_options_config() -> TaskOptionsConfig {
        TaskOptionsConfig {
            merge_args: None,
            merge_deps: None,
            merge_inputs: None,
            merge_outputs: None,
            retry_count: Some(1),
            run_in_ci: Some(true),
            run_from_workspace_root: None,
        }
    }

    fn create_expanded_task(
        target: TargetID,
        config: &TaskConfig,
        workspace_root: &Path,
        project_path: &str,
    ) -> Result<Task, ProjectError> {
        let project_root = workspace_root.join(project_path);

        let mut task = Task::from_config(target, config);
        task.expand_inputs(workspace_root, &project_root)?;
        task.expand_outputs(workspace_root, &project_root)?;

        Ok(task)
    }

    #[test]
    fn inherits_global_tasks() {
        let root_dir = get_fixture_root();
        let project = Project::new(
            "id",
            "tasks/no-tasks",
            &root_dir,
            &GlobalProjectConfig {
                file_groups: None,
                tasks: Some(HashMap::from([(
                    String::from("standard"),
                    mock_task_config("cmd"),
                )])),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    depends_on: None,
                    file_groups: None,
                    project: None,
                    tasks: Some(HashMap::new()),
                }),
                dir: root_dir.join("tasks/no-tasks").canonicalize().unwrap(),
                file_groups: HashMap::new(),
                location: String::from("tasks/no-tasks"),
                package_json: None,
                tasks: HashMap::from([(
                    String::from("standard"),
                    Task::from_config(
                        Target::format("id", "standard").unwrap(),
                        &mock_task_config("cmd")
                    )
                )]),
            }
        );
    }

    #[test]
    fn merges_with_global_tasks() {
        let root_dir = get_fixture_root();
        let project = Project::new(
            "id",
            "tasks/basic",
            &root_dir,
            &GlobalProjectConfig {
                file_groups: None,
                tasks: Some(HashMap::from([(
                    String::from("standard"),
                    mock_task_config("cmd"),
                )])),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    depends_on: None,
                    file_groups: None,
                    project: None,
                    tasks: Some(HashMap::from([(
                        String::from("lint"),
                        mock_task_config("eslint"),
                    )])),
                }),
                dir: root_dir.join("tasks/basic").canonicalize().unwrap(),
                file_groups: HashMap::new(),
                location: String::from("tasks/basic"),
                package_json: None,
                tasks: HashMap::from([
                    (
                        String::from("standard"),
                        Task::from_config(
                            Target::format("id", "standard").unwrap(),
                            &mock_task_config("cmd")
                        )
                    ),
                    (
                        String::from("lint"),
                        Task::from_config(
                            Target::format("id", "lint").unwrap(),
                            &mock_task_config("eslint")
                        )
                    )
                ]),
            }
        );
    }

    #[test]
    fn strategy_replace() {
        let root_dir = get_fixture_root();
        let project_path = "tasks/merge-replace";
        let project = Project::new(
            "id",
            project_path,
            &root_dir,
            &GlobalProjectConfig {
                file_groups: None,
                tasks: Some(HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(vec!["--a".to_owned()]),
                        command: Some(String::from("standard")),
                        deps: Some(vec!["a:standard".to_owned()]),
                        inputs: Some(vec!["a.*".to_owned()]),
                        outputs: Some(vec!["a.ts".to_owned()]),
                        options: Some(stub_global_task_options_config()),
                        type_of: None,
                    },
                )])),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    depends_on: None,
                    file_groups: None,
                    project: None,
                    tasks: Some(HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(vec!["--b".to_owned()]),
                            command: Some(String::from("newcmd")),
                            deps: Some(vec!["b:standard".to_owned()]),
                            inputs: Some(vec!["b.*".to_owned()]),
                            outputs: Some(vec!["b.ts".to_owned()]),
                            options: Some(mock_local_task_options_config(
                                TaskMergeStrategy::Replace
                            )),
                            type_of: Some(TaskType::Shell),
                        }
                    )])),
                }),
                dir: root_dir.join(project_path).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                location: String::from(project_path),
                package_json: None,
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        &TaskConfig {
                            args: Some(vec!["--b".to_owned()]),
                            command: Some(String::from("newcmd")),
                            deps: Some(vec!["b:standard".to_owned()]),
                            inputs: Some(vec!["b.*".to_owned()]),
                            outputs: Some(vec!["b.ts".to_owned()]),
                            options: Some(mock_merged_task_options_config(
                                TaskMergeStrategy::Replace
                            )),
                            type_of: Some(TaskType::Shell),
                        },
                        &root_dir,
                        project_path
                    )
                    .unwrap()
                )]),
            }
        );
    }

    #[test]
    fn strategy_append() {
        let root_dir = get_fixture_root();
        let project_path = "tasks/merge-append";
        let project = Project::new(
            "id",
            project_path,
            &root_dir,
            &GlobalProjectConfig {
                file_groups: None,
                tasks: Some(HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(vec!["--a".to_owned()]),
                        command: Some(String::from("standard")),
                        deps: Some(vec!["a:standard".to_owned()]),
                        inputs: Some(vec!["a.*".to_owned()]),
                        outputs: Some(vec!["a.ts".to_owned()]),
                        options: Some(stub_global_task_options_config()),
                        type_of: None,
                    },
                )])),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    depends_on: None,
                    file_groups: None,
                    project: None,
                    tasks: Some(HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(vec!["--b".to_owned()]),
                            command: None,
                            deps: Some(vec!["b:standard".to_owned()]),
                            inputs: Some(vec!["b.*".to_owned()]),
                            outputs: Some(vec!["b.ts".to_owned()]),
                            options: Some(mock_local_task_options_config(
                                TaskMergeStrategy::Append
                            )),
                            type_of: Some(TaskType::Shell),
                        }
                    )])),
                }),
                dir: root_dir.join(project_path).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                location: String::from(project_path),
                package_json: None,
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        &TaskConfig {
                            args: Some(vec!["--a".to_owned(), "--b".to_owned()]),
                            command: Some(String::from("standard")),
                            deps: Some(vec!["a:standard".to_owned(), "b:standard".to_owned()]),
                            inputs: Some(vec!["a.*".to_owned(), "b.*".to_owned()]),
                            outputs: Some(vec!["a.ts".to_owned(), "b.ts".to_owned()]),
                            options: Some(mock_merged_task_options_config(
                                TaskMergeStrategy::Append
                            )),
                            type_of: Some(TaskType::Shell),
                        },
                        &root_dir,
                        project_path
                    )
                    .unwrap()
                )]),
            }
        );
    }

    #[test]
    fn strategy_prepend() {
        let root_dir = get_fixture_root();
        let project_path = "tasks/merge-prepend";
        let project = Project::new(
            "id",
            project_path,
            &root_dir,
            &GlobalProjectConfig {
                file_groups: None,
                tasks: Some(HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(vec!["--a".to_owned()]),
                        command: Some(String::from("standard")),
                        deps: Some(vec!["a:standard".to_owned()]),
                        inputs: Some(vec!["a.*".to_owned()]),
                        outputs: Some(vec!["a.ts".to_owned()]),
                        options: Some(stub_global_task_options_config()),
                        type_of: None,
                    },
                )])),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    depends_on: None,
                    file_groups: None,
                    project: None,
                    tasks: Some(HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(vec!["--b".to_owned()]),
                            command: Some(String::from("newcmd")),
                            deps: Some(vec!["b:standard".to_owned()]),
                            inputs: Some(vec!["b.*".to_owned()]),
                            outputs: Some(vec!["b.ts".to_owned()]),
                            options: Some(mock_local_task_options_config(
                                TaskMergeStrategy::Prepend
                            )),
                            type_of: Some(TaskType::Shell),
                        }
                    )])),
                }),
                dir: root_dir.join(project_path).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                location: String::from(project_path),
                package_json: None,
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        &TaskConfig {
                            args: Some(vec!["--b".to_owned(), "--a".to_owned()]),
                            command: Some(String::from("newcmd")),
                            deps: Some(vec!["b:standard".to_owned(), "a:standard".to_owned()]),
                            inputs: Some(vec!["b.*".to_owned(), "a.*".to_owned()]),
                            outputs: Some(vec!["b.ts".to_owned(), "a.ts".to_owned()]),
                            options: Some(mock_merged_task_options_config(
                                TaskMergeStrategy::Prepend
                            )),
                            type_of: Some(TaskType::Shell),
                        },
                        &root_dir,
                        project_path
                    )
                    .unwrap()
                )]),
            }
        );
    }

    #[test]
    fn strategy_all() {
        let root_dir = get_fixture_root();
        let project_path = "tasks/merge-all-strategies";
        let project = Project::new(
            "id",
            project_path,
            &root_dir,
            &GlobalProjectConfig {
                file_groups: None,
                tasks: Some(HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(vec!["--a".to_owned()]),
                        command: Some(String::from("standard")),
                        deps: Some(vec!["a:standard".to_owned()]),
                        inputs: Some(vec!["a.*".to_owned()]),
                        outputs: Some(vec!["a.ts".to_owned()]),
                        options: Some(stub_global_task_options_config()),
                        type_of: None,
                    },
                )])),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    depends_on: None,
                    file_groups: None,
                    project: None,
                    tasks: Some(HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(vec!["--b".to_owned()]),
                            command: None,
                            deps: Some(vec!["b:standard".to_owned()]),
                            inputs: Some(vec!["b.*".to_owned()]),
                            outputs: Some(vec!["b.ts".to_owned()]),
                            options: Some(TaskOptionsConfig {
                                merge_args: Some(TaskMergeStrategy::Append),
                                merge_deps: Some(TaskMergeStrategy::Prepend),
                                merge_inputs: Some(TaskMergeStrategy::Replace),
                                merge_outputs: Some(TaskMergeStrategy::Append),
                                retry_count: None,
                                run_in_ci: None,
                                run_from_workspace_root: None,
                            }),
                            type_of: None,
                        }
                    )])),
                }),
                dir: root_dir.join(project_path).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                location: String::from(project_path),
                package_json: None,
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        &TaskConfig {
                            args: Some(vec!["--a".to_owned(), "--b".to_owned()]),
                            command: Some(String::from("standard")),
                            deps: Some(vec!["b:standard".to_owned(), "a:standard".to_owned()]),
                            inputs: Some(vec!["b.*".to_owned()]),
                            outputs: Some(vec!["a.ts".to_owned(), "b.ts".to_owned()]),
                            options: Some(TaskOptionsConfig {
                                merge_args: Some(TaskMergeStrategy::Append),
                                merge_deps: Some(TaskMergeStrategy::Prepend),
                                merge_inputs: Some(TaskMergeStrategy::Replace),
                                merge_outputs: Some(TaskMergeStrategy::Append),
                                retry_count: Some(1),
                                run_in_ci: Some(true),
                                run_from_workspace_root: None,
                            }),
                            type_of: Some(TaskType::Npm),
                        },
                        &root_dir,
                        project_path
                    )
                    .unwrap()
                )]),
            }
        );
    }
}
