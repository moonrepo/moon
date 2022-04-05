use moon_config::package::PackageJson;
use moon_config::{
    GlobalProjectConfig, ProjectConfig, ProjectMetadataConfig, ProjectType, TargetID, TaskConfig,
    TaskMergeStrategy, TaskOptionsConfig, TaskType,
};
use moon_project::{EnvVars, FileGroup, Project, ProjectError, Target, Task};
use moon_utils::string_vec;
use moon_utils::test::{get_fixtures_dir, get_fixtures_root};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::path::Path;

fn mock_file_groups() -> HashMap<String, FileGroup> {
    HashMap::from([(
        String::from("sources"),
        FileGroup::new("sources", string_vec!["src/**/*"]),
    )])
}

fn mock_global_project_config() -> GlobalProjectConfig {
    GlobalProjectConfig {
        file_groups: HashMap::from([(String::from("sources"), string_vec!["src/**/*"])]),
        tasks: HashMap::new(),
        schema: String::new(),
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
            file_groups: mock_file_groups(),
            source: String::from("projects/no-config"),
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
            config: Some(ProjectConfig::default()),
            root: workspace_root.join("projects/empty-config"),
            file_groups: mock_file_groups(),
            source: String::from("projects/empty-config"),
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
    let mut file_groups = mock_file_groups();
    file_groups.insert(
        String::from("tests"),
        FileGroup::new("tests", string_vec!["**/*_test.rs"]),
    );

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: Some(ProjectConfig {
                depends_on: string_vec!["noConfig"],
                file_groups: HashMap::from([(String::from("tests"), string_vec!["**/*_test.rs"])]),
                ..ProjectConfig::default()
            }),
            root: project_root,
            file_groups,
            source: String::from("projects/basic"),
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
                project: Some(ProjectMetadataConfig {
                    type_of: ProjectType::Library,
                    name: String::from("Advanced"),
                    description: String::from("Advanced example."),
                    owner: String::from("Batman"),
                    maintainers: string_vec!["Bruce Wayne"],
                    channel: String::from("#batcave"),
                }),
                ..ProjectConfig::default()
            }),
            root: workspace_root.join("projects/advanced"),
            file_groups: mock_file_groups(),
            source: String::from("projects/advanced"),
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
            file_groups: HashMap::from([(String::from("tests"), string_vec!["tests/**/*"])]),
            tasks: HashMap::new(),
            schema: String::new(),
        },
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: Some(ProjectConfig {
                depends_on: string_vec!["noConfig"],
                file_groups: HashMap::from([(String::from("tests"), string_vec!["**/*_test.rs"])]),
                ..ProjectConfig::default()
            }),
            root: workspace_root.join("projects/basic"),
            file_groups: HashMap::from([(
                String::from("tests"),
                FileGroup::new("tests", string_vec!["**/*_test.rs"],)
            )]),
            source: String::from("projects/basic"),
            tasks: HashMap::new(),
        }
    );
}

#[tokio::test]
async fn has_package_json() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "package-json",
        "projects/package-json",
        &workspace_root,
        &mock_global_project_config(),
    )
    .unwrap();

    assert_eq!(
        project.load_package_json().await.unwrap().unwrap(),
        PackageJson {
            path: workspace_root.join("projects/package-json/package.json"),
            name: Some(String::from("npm-example")),
            version: Some(String::from("1.2.3")),
            scripts: Some(BTreeMap::from([("build".to_owned(), "babel".to_owned())])),
            ..PackageJson::default()
        }
    );
}

mod tasks {
    use super::*;
    use moon_project::test::{
        create_expanded_task as create_expanded_task_internal, create_file_groups_config,
    };
    use moon_utils::test::wrap_glob;
    use pretty_assertions::assert_eq;

    fn mock_task_config(command: &str) -> TaskConfig {
        TaskConfig {
            command: Some(command.to_owned()),
            ..TaskConfig::default()
        }
    }

    fn mock_merged_task_options_config(strategy: TaskMergeStrategy) -> TaskOptionsConfig {
        TaskOptionsConfig {
            merge_args: Some(strategy.clone()),
            merge_deps: Some(strategy.clone()),
            merge_env: Some(strategy.clone()),
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
            merge_env: Some(strategy.clone()),
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
            merge_env: None,
            merge_inputs: None,
            merge_outputs: None,
            retry_count: Some(1),
            run_in_ci: Some(true),
            run_from_workspace_root: None,
        }
    }

    fn stub_global_env_vars() -> EnvVars {
        HashMap::from([
            ("GLOBAL".to_owned(), "1".to_owned()),
            ("KEY".to_owned(), "a".to_owned()),
        ])
    }

    fn create_expanded_task(
        target: TargetID,
        config: TaskConfig,
        workspace_root: &Path,
        project_source: &str,
    ) -> Result<Task, ProjectError> {
        let project_root = workspace_root.join(project_source);
        let mut task =
            create_expanded_task_internal(workspace_root, &project_root, Some(config)).unwrap();

        task.target = target;

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
                file_groups: HashMap::new(),
                tasks: HashMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                schema: String::new(),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig::default()),
                root: workspace_root
                    .join("tasks/no-tasks")
                    .canonicalize()
                    .unwrap(),
                file_groups: HashMap::new(),
                source: String::from("tasks/no-tasks"),
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
                file_groups: HashMap::new(),
                tasks: HashMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                schema: String::new(),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    tasks: HashMap::from([
                        (String::from("build"), mock_task_config("webpack")),
                        (String::from("test"), mock_task_config("jest")),
                        (String::from("lint"), mock_task_config("eslint"))
                    ]),
                    ..ProjectConfig::default()
                }),
                root: workspace_root.join("tasks/basic").canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from("tasks/basic"),
                tasks: HashMap::from([
                    (
                        String::from("build"),
                        Task::from_config(
                            Target::format("id", "build").unwrap(),
                            &mock_task_config("webpack")
                        )
                    ),
                    (
                        String::from("standard"),
                        Task::from_config(
                            Target::format("id", "standard").unwrap(),
                            &mock_task_config("cmd")
                        )
                    ),
                    (
                        String::from("test"),
                        Task::from_config(
                            Target::format("id", "test").unwrap(),
                            &mock_task_config("jest")
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
                file_groups: HashMap::new(),
                tasks: HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: TaskType::Node,
                    },
                )]),
                schema: String::new(),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    tasks: HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Replace),
                            type_of: TaskType::Shell,
                        }
                    )]),
                    ..ProjectConfig::default()
                }),
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Replace),
                            type_of: TaskType::Shell,
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
                file_groups: HashMap::new(),
                tasks: HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: TaskType::Node,
                    },
                )]),
                schema: String::new(),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    tasks: HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: None,
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Append),
                            type_of: TaskType::Shell,
                        }
                    )]),
                    ..ProjectConfig::default()
                }),
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        TaskConfig {
                            args: Some(string_vec!["--a", "--b"]),
                            command: Some(String::from("standard")),
                            deps: Some(string_vec!["a:standard", "b:standard"]),
                            env: Some(HashMap::from([
                                ("GLOBAL".to_owned(), "1".to_owned()),
                                ("KEY".to_owned(), "b".to_owned())
                            ])),
                            inputs: Some(string_vec!["a.*", "b.*"]),
                            outputs: Some(string_vec!["a.ts", "b.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Append),
                            type_of: TaskType::Shell,
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
                file_groups: HashMap::new(),
                tasks: HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: TaskType::Node,
                    },
                )]),
                schema: String::new(),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    tasks: HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Prepend),
                            type_of: TaskType::Shell,
                        }
                    )]),
                    ..ProjectConfig::default()
                }),
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        TaskConfig {
                            args: Some(string_vec!["--b", "--a"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard", "a:standard"]),
                            env: Some(HashMap::from([
                                ("GLOBAL".to_owned(), "1".to_owned()),
                                ("KEY".to_owned(), "a".to_owned())
                            ])),
                            inputs: Some(string_vec!["b.*", "a.*"]),
                            outputs: Some(string_vec!["b.ts", "a.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Prepend),
                            type_of: TaskType::Shell,
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
                file_groups: HashMap::new(),
                tasks: HashMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: TaskType::Node,
                    },
                )]),
                schema: String::new(),
            },
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: Some(ProjectConfig {
                    tasks: HashMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: None,
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: TaskOptionsConfig {
                                merge_args: Some(TaskMergeStrategy::Append),
                                merge_deps: Some(TaskMergeStrategy::Prepend),
                                merge_env: Some(TaskMergeStrategy::Replace),
                                merge_inputs: Some(TaskMergeStrategy::Replace),
                                merge_outputs: Some(TaskMergeStrategy::Append),
                                retry_count: None,
                                run_in_ci: None,
                                run_from_workspace_root: None,
                            },
                            type_of: TaskType::Node,
                        }
                    )]),
                    ..ProjectConfig::default()
                }),
                root: workspace_root.join(project_source).canonicalize().unwrap(),
                file_groups: HashMap::new(),
                source: String::from(project_source),
                tasks: HashMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        TaskConfig {
                            args: Some(string_vec!["--a", "--b"]),
                            command: Some(String::from("standard")),
                            deps: Some(string_vec!["b:standard", "a:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["a.ts", "b.ts"]),
                            options: TaskOptionsConfig {
                                merge_args: Some(TaskMergeStrategy::Append),
                                merge_deps: Some(TaskMergeStrategy::Prepend),
                                merge_env: Some(TaskMergeStrategy::Replace),
                                merge_inputs: Some(TaskMergeStrategy::Replace),
                                merge_outputs: Some(TaskMergeStrategy::Append),
                                retry_count: Some(1),
                                run_in_ci: Some(true),
                                run_from_workspace_root: None,
                            },
                            type_of: TaskType::Node,
                        },
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
            }
        );
    }

    mod expands_deps {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn resolves_self_scope() {
            let project = Project::new(
                "id",
                "self",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(
                project.tasks.get("lint").unwrap().deps,
                string_vec!["id:clean", "id:build"]
            );
        }

        #[test]
        fn resolves_deps_scope() {
            let project = Project::new(
                "id",
                "deps",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(
                project.tasks.get("build").unwrap().deps,
                string_vec!["foo:build", "bar:build", "baz:build"]
            );
        }

        #[test]
        #[should_panic(expected = "Target(NoProjectAllInTaskDeps(\":build\"))")]
        fn errors_for_all_scope() {
            Project::new(
                "id",
                "all",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
            )
            .unwrap();
        }
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
                    file_groups: create_file_groups_config(),
                    tasks: HashMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(string_vec![
                                "--dirs",
                                "@dirs(static)",
                                "--files",
                                "@files(static)",
                                "--globs",
                                "@globs(globs)",
                                "--root",
                                "@root(static)",
                            ]),
                            command: Some(String::from("test")),
                            ..TaskConfig::default()
                        },
                    )]),
                    schema: String::new(),
                },
            )
            .unwrap();

            assert_eq!(
                *project.tasks.get("test").unwrap().args,
                if env::consts::OS == "windows" {
                    vec![
                        "--dirs",
                        ".\\dir",
                        ".\\dir\\subdir",
                        "--files",
                        ".\\file.ts",
                        ".\\dir\\other.tsx",
                        ".\\dir\\subdir\\another.ts",
                        "--globs",
                        ".\\**\\*.{ts,tsx}",
                        ".\\*.js",
                        "--root",
                        ".\\dir",
                    ]
                } else {
                    vec![
                        "--dirs",
                        "./dir",
                        "./dir/subdir",
                        "--files",
                        "./file.ts",
                        "./dir/other.tsx",
                        "./dir/subdir/another.ts",
                        "--globs",
                        "./**/*.{ts,tsx}",
                        "./*.js",
                        "--root",
                        "./dir",
                    ]
                },
            )
        }

        #[test]
        fn expands_args_from_workspace() {
            let workspace_root = get_fixtures_root();
            let project_root = workspace_root.join("base/files-and-dirs");
            let project = Project::new(
                "id",
                "base/files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: HashMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(string_vec![
                                "--dirs",
                                "@dirs(static)",
                                "--files",
                                "@files(static)",
                                "--globs",
                                "@globs(globs)",
                                "--root",
                                "@root(static)",
                            ]),
                            command: Some(String::from("test")),
                            options: TaskOptionsConfig {
                                run_from_workspace_root: Some(true),
                                ..TaskOptionsConfig::default()
                            },
                            ..TaskConfig::default()
                        },
                    )]),
                    schema: String::new(),
                },
            )
            .unwrap();

            assert_eq!(
                *project.tasks.get("test").unwrap().args,
                vec![
                    "--dirs",
                    project_root.join("dir").to_str().unwrap(),
                    project_root.join("dir/subdir").to_str().unwrap(),
                    "--files",
                    project_root.join("file.ts").to_str().unwrap(),
                    project_root.join("dir/other.tsx").to_str().unwrap(),
                    project_root.join("dir/subdir/another.ts").to_str().unwrap(),
                    "--globs",
                    project_root.join("**/*.{ts,tsx}").to_str().unwrap(),
                    project_root.join("*.js").to_str().unwrap(),
                    "--root",
                    project_root.join("dir").to_str().unwrap(),
                ],
            )
        }

        #[test]
        fn expands_args_with_vars() {
            let workspace_root = get_fixtures_root();
            let project_root = workspace_root.join("base/files-and-dirs");
            let project = Project::new(
                "id",
                "base/files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: HashMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(string_vec![
                                "some/$unknown/var", // Unknown
                                "--pid",
                                "$project/foo", // At start
                                "--proot",
                                "$projectRoot", // Alone
                                "--psource",
                                "foo/$projectSource", // At end
                                "--target",
                                "foo/$target/bar", // In middle
                                "--tid=$task",     // As an arg
                                "--wsroot",
                                "$workspaceRoot" // Alone
                            ]),
                            command: Some(String::from("test")),
                            ..TaskConfig::default()
                        },
                    )]),
                    schema: String::new(),
                },
            )
            .unwrap();

            assert_eq!(
                *project.tasks.get("test").unwrap().args,
                vec![
                    "some/$unknown/var",
                    "--pid",
                    "id/foo",
                    "--proot",
                    project_root.to_str().unwrap(),
                    "--psource",
                    // This is wonky but also still valid
                    if env::consts::OS == "windows" {
                        "foo/base\\files-and-dirs"
                    } else {
                        "foo/base/files-and-dirs"
                    },
                    "--target",
                    "foo/id:test/bar",
                    "--tid=test",
                    "--wsroot",
                    workspace_root.to_str().unwrap(),
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
                    file_groups: create_file_groups_config(),
                    tasks: HashMap::from([(
                        String::from("test"),
                        TaskConfig {
                            command: Some(String::from("test")),
                            inputs: Some(string_vec![
                                "file.ts",
                                "@dirs(static)",
                                "@files(static)",
                                "@globs(globs)",
                                "@root(static)",
                                "/package.json",
                            ]),
                            ..TaskConfig::default()
                        },
                    )]),
                    schema: String::new(),
                },
            )
            .unwrap();

            let task = project.tasks.get("test").unwrap();

            assert_eq!(
                task.input_globs,
                vec![
                    wrap_glob(&project_root.join("**/*.{ts,tsx}")).to_string_lossy(),
                    wrap_glob(&project_root.join("*.js")).to_string_lossy()
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

mod workspace {
    use super::*;
    use moon_project::test::create_expanded_task;

    mod inherited_tasks {
        use super::*;

        fn mock_global_project_config() -> GlobalProjectConfig {
            GlobalProjectConfig {
                file_groups: HashMap::new(),
                tasks: HashMap::from([
                    (
                        String::from("a"),
                        TaskConfig {
                            command: Some(String::from("a")),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        String::from("b"),
                        TaskConfig {
                            command: Some(String::from("b")),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        String::from("c"),
                        TaskConfig {
                            command: Some(String::from("c")),
                            ..TaskConfig::default()
                        },
                    ),
                ]),
                schema: String::new(),
            }
        }

        fn get_project_task_ids(project: Project) -> Vec<String> {
            let mut ids = project.tasks.into_keys().collect::<Vec<String>>();
            ids.sort();
            ids
        }

        #[test]
        fn include() {
            let project = Project::new(
                "id",
                "include",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["a", "c"])
        }

        #[test]
        fn include_none() {
            let project = Project::new(
                "id",
                "include-none",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec![])
        }

        #[test]
        fn exclude() {
            let project = Project::new(
                "id",
                "exclude",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["b"])
        }

        #[test]
        fn exclude_all() {
            let project = Project::new(
                "id",
                "exclude-all",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec![])
        }

        #[test]
        fn exclude_none() {
            let project = Project::new(
                "id",
                "exclude-none",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["a", "b", "c"])
        }

        #[test]
        fn rename() {
            let project = Project::new(
                "id",
                "rename",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(
                get_project_task_ids(project),
                string_vec!["bar", "baz", "foo"]
            )
        }

        #[test]
        fn rename_merge() {
            let workspace_root = get_fixtures_dir("task-inheritance");
            let project = Project::new(
                "id",
                "rename-merge",
                &workspace_root,
                &mock_global_project_config(),
            )
            .unwrap();

            let mut task =
                create_expanded_task(&workspace_root, &workspace_root.join("rename-merge"), None)
                    .unwrap();
            task.target = "id:foo".to_owned();
            task.command = "a".to_owned();
            task.args.push("renamed-and-merge-foo".to_owned());

            assert_eq!(*project.get_task("foo").unwrap(), task);

            assert_eq!(get_project_task_ids(project), string_vec!["b", "c", "foo"]);
        }

        #[test]
        fn include_exclude() {
            let project = Project::new(
                "id",
                "include-exclude",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["a"])
        }

        #[test]
        fn include_exclude_rename() {
            let project = Project::new(
                "id",
                "include-exclude-rename",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["only"])
        }
    }
}
