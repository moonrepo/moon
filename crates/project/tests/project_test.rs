use moon_config::{
    GlobalProjectConfig, PackageJson, ProjectConfig, ProjectMetadataConfig, ProjectType, TargetID,
    TaskConfig, TaskMergeStrategy, TaskOptionsConfig, TaskType,
};
use moon_project::{FileGroup, Project, ProjectError, Target, Task, TokenResolver};
use moon_utils::test::{get_fixtures_dir, get_fixtures_root};
use std::collections::HashMap;
use std::path::Path;

fn mock_file_groups(root: &Path) -> HashMap<String, FileGroup> {
    HashMap::from([(
        String::from("sources"),
        FileGroup::new("sources", vec![String::from("src/**/*")], root),
    )])
}

fn mock_global_project_config() -> GlobalProjectConfig {
    GlobalProjectConfig {
        file_groups: Some(HashMap::from([(
            String::from("sources"),
            vec![String::from("src/**/*")],
        )])),
        tasks: None,
    }
}

#[test]
#[should_panic(expected = "MissingProject(\"projects/missing\")")]
fn doesnt_exist() {
    Project::new(
        "missing",
        "projects/missing",
        &get_fixtures_root(),
        &mock_global_project_config(),
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
        &mock_global_project_config(),
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("no-config"),
            config: None,
            root: workspace_root.join("projects/no-config"),
            file_groups: mock_file_groups(&workspace_root.join("projects/no-config")),
            source: String::from("projects/no-config"),
            package_json: None,
            tasks: HashMap::new(),
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
            root: workspace_root.join("projects/empty-config"),
            file_groups: mock_file_groups(&workspace_root.join("projects/empty-config")),
            source: String::from("projects/empty-config"),
            package_json: None,
            tasks: HashMap::new(),
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
        &mock_global_project_config(),
    )
    .unwrap();
    let project_root = workspace_root.join("projects/basic");

    // Merges with global
    let mut file_groups = mock_file_groups(&project_root);
    file_groups.insert(
        String::from("tests"),
        FileGroup::new("tests", vec![String::from("**/*_test.rs")], &project_root),
    );

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
            root: project_root,
            file_groups,
            source: String::from("projects/basic"),
            package_json: None,
            tasks: HashMap::new(),
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
            root: workspace_root.join("projects/advanced"),
            file_groups: mock_file_groups(&workspace_root.join("projects/advanced")),
            source: String::from("projects/advanced"),
            package_json: None,
            tasks: HashMap::new(),
        }
    );
}

#[test]
fn overrides_global_file_groups() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "basic",
        "projects/basic",
        &workspace_root,
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
            root: workspace_root.join("projects/basic"),
            file_groups: HashMap::from([(
                String::from("tests"),
                FileGroup::new(
                    "tests",
                    vec![String::from("**/*_test.rs")],
                    &workspace_root.join("projects/basic")
                )
            )]),
            source: String::from("projects/basic"),
            package_json: None,
            tasks: HashMap::new(),
        }
    );
}

#[test]
fn has_package_json() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "package-json",
        "projects/package-json",
        &workspace_root,
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
            root: workspace_root.join("projects/package-json"),
            file_groups: mock_file_groups(&workspace_root.join("projects/package-json")),
            source: String::from("projects/package-json"),
            package_json: Some(PackageJson::from(json).unwrap()),
            tasks: HashMap::new(),
        }
    );
}

mod tasks {
    use super::*;
    use moon_project::test::{create_file_groups, create_file_groups_config};
    use moon_project::TokenSharedData;
    use pretty_assertions::assert_eq;

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
        project_source: &str,
    ) -> Result<Task, ProjectError> {
        let project_root = workspace_root.join(project_source);
        let file_groups = create_file_groups(&project_root);
        let metadata = TokenSharedData::new(&file_groups, workspace_root, &project_root);

        let mut task = Task::from_config(target, config);
        task.expand_inputs(TokenResolver::for_inputs(&metadata))?;
        task.expand_outputs(TokenResolver::for_outputs(&metadata))?;
        task.expand_args(TokenResolver::for_args(&metadata))?; // Must be last

        Ok(task)
    }

    #[test]
    fn inherits_global_tasks() {
        let workspace_root = get_fixtures_root();
        let project = Project::new(
            "id",
            "tasks/no-tasks",
            &workspace_root,
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
                root: workspace_root
                    .join("tasks/no-tasks")
                    .canonicalize()
                    .unwrap(),
                file_groups: HashMap::new(),
                source: String::from("tasks/no-tasks"),
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
        let workspace_root = get_fixtures_root();
        let project = Project::new(
            "id",
            "tasks/basic",
            &workspace_root,
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
                root: workspace_root.join("tasks/basic").canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from("tasks/basic"),
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
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-replace";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
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
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
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
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
            }
        );
    }

    #[test]
    fn strategy_append() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-append";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
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
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
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
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
            }
        );
    }

    #[test]
    fn strategy_prepend() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-prepend";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
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
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
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
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
            }
        );
    }

    #[test]
    fn strategy_all() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-all-strategies";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
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
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
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
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
            }
        );
    }

    mod tokens {
        use super::*;
        use pretty_assertions::assert_eq;
        use std::collections::HashSet;
        use std::path::PathBuf;

        #[test]
        fn expands_args() {
            let project = Project::new(
                "id",
                "base/files-and-dirs",
                &get_fixtures_root(),
                &GlobalProjectConfig {
                    file_groups: Some(create_file_groups_config()),
                    tasks: Some(HashMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(vec![
                                "--dirs".to_owned(),
                                "@dirs(static)".to_owned(),
                                "--files".to_owned(),
                                "@files(static)".to_owned(),
                                "--globs".to_owned(),
                                "@globs(globs)".to_owned(),
                                "--root".to_owned(),
                                "@root(static)".to_owned(),
                            ]),
                            command: Some(String::from("test")),
                            deps: None,
                            inputs: None,
                            outputs: None,
                            options: None,
                            type_of: None,
                        },
                    )])),
                },
            )
            .unwrap();

            assert_eq!(
                *project.tasks.get("test").unwrap().args,
                vec![
                    "--dirs".to_owned(),
                    "dir".to_owned(),
                    "dir/subdir".to_owned(),
                    "--files".to_owned(),
                    "file.ts".to_owned(),
                    "dir/other.tsx".to_owned(),
                    "dir/subdir/another.ts".to_owned(),
                    "--globs".to_owned(),
                    "**/*.{ts,tsx}".to_owned(),
                    "*.js".to_owned(),
                    "--root".to_owned(),
                    "dir".to_owned(),
                ],
            )
        }

        #[test]
        fn expands_inputs() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let project = Project::new(
                "id",
                "files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: Some(create_file_groups_config()),
                    tasks: Some(HashMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: None,
                            command: Some(String::from("test")),
                            deps: None,
                            inputs: Some(vec![
                                "file.ts".to_owned(),
                                "@dirs(static)".to_owned(),
                                "@files(static)".to_owned(),
                                "@globs(globs)".to_owned(),
                                "@root(static)".to_owned(),
                                "/package.json".to_owned(),
                            ]),
                            outputs: None,
                            options: None,
                            type_of: None,
                        },
                    )])),
                },
            )
            .unwrap();

            let task = project.tasks.get("test").unwrap();

            assert_eq!(
                task.input_globs,
                vec![
                    project_root.join("**/*.{ts,tsx}").to_string_lossy(),
                    project_root.join("*.js").to_string_lossy()
                ],
            );

            let a: HashSet<PathBuf> =
                HashSet::from_iter(task.input_paths.iter().map(PathBuf::from));
            let b: HashSet<PathBuf> = HashSet::from_iter(
                vec![
                    project_root.join("file.ts"),
                    project_root.join("dir"),
                    project_root.join("dir/subdir"),
                    project_root.join("file.ts"),
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts"),
                    workspace_root.join("package.json"),
                ]
                .iter()
                .map(PathBuf::from),
            );

            assert_eq!(a, b);
        }
    }
}
